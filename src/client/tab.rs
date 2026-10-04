use std::{format, io::Read, path::Path, process::{Command, Stdio}, thread, vec};

use file_format::{FileFormat, Kind};
use indexmap::IndexMap;
use mime_guess::mime;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

use crate::{client::{AtomicClientState, Scanning, client_ext::file::ClientFile}, common::log};

fn ffprobe_duration(path: &str) -> Option<u128> {
	let Ok(info) = ffprobe::ffprobe(path) else { return None };
	if info.streams.par_iter().any(|stream| stream.codec_type == Option::Some("audio".to_string())) {
		if let Some(duration) = info.format.get_duration() {
			Some(duration.as_millis())
		} else {
			None
		}
	} else {
		None
	}
}

fn ffmpeg_duration(path: &str) -> Option<u128> {
	let child = match Command::new("ffmpeg").args([
		"-loglevel", "-8",
		"-i", path,
		"-f", "u8",
		"-ac", "1",
		"-ar", "1000",
		"-"
	]).stdout(Stdio::piped()).spawn() {
		Ok(child) => child,
		Err(_) => { return None; }
	};
	let mut buf = vec![];
	let _ = child.stdout.unwrap().read_to_end(&mut buf);
	Some(buf.len() as u128)
}

fn add_duration(client_state: AtomicClientState, tab: String) {
	let files = {
		let client_state = client_state.read();
		let Some(files) = client_state.file_tabs.get(&tab) else { return; };
		files.clone()
	};
	let mut new_files = IndexMap::new();
	for (filename, info) in files {
		let longpath = Path::new(&tab).join(filename.clone());
		let filepath = longpath.into_os_string().into_string().unwrap();
		let mut info = info.clone();

		let millis = match ffprobe_duration(&filepath) {
			Some(duration) => duration,
			None => match ffmpeg_duration(&filepath) {
				Some(duration) => duration,
				None => {
					new_files.insert(filename.clone(), info);
					continue;
				}
			}
		};

		let mut duration_str = String::new();
		let hours = millis / (1000 * 60 * 60);
		let minutes = millis / (1000 * 60) - hours * 60;
		let seconds = millis / 1000 - hours * 60 * 60 - minutes * 60;
		let millis = millis - ((hours * 60 + minutes) * 60 + seconds) * 1000;
		let mut unit = "";
		if hours > 0 {
			duration_str += &format!("{:0>2}:", hours.to_string());
		}
		if minutes > 0 || !duration_str.is_empty() {
			duration_str += &format!("{:0>2}:", minutes.to_string());
		}
		if duration_str.is_empty() && seconds > 0 {
			duration_str += &format!("{}.", seconds.to_string());
			unit = " s";
		} else if !duration_str.is_empty() {
			duration_str += &format!("{:0>2}.", seconds.to_string());
		}
		if duration_str.is_empty() {
			duration_str += &format!("{}", millis.to_string());
			unit = " ms";
		} else {
			duration_str += &format!("{:0>3}", millis.to_string());
		}
		duration_str += unit;
		info.duration = duration_str;
		new_files.insert(filename.clone(), info);
	}
	let mut client_state = client_state.write();
	if let Some(files) = client_state.file_tabs.get_mut(&tab) {
		*files = new_files;
		client_state.redrawer.notify();
	}
}

fn scan_tab(atomic_client_state: AtomicClientState, index: usize) -> Result<(), Box<dyn std::error::Error>> {
	let client_state = atomic_client_state.clone();
	let client_state = client_state.read();
	let fast_scan = client_state.config.fast_scan;
	if index >= client_state.file_tabs.len() {
		return Ok(());
	}
	let (tab, old_files) = client_state.file_tabs.get_index(index).unwrap();
	let tab = tab.clone();
	let old_files = old_files.clone();
	drop(client_state);
	let mut files = IndexMap::new();
	let path = Path::new(tab.as_str());
	if path.is_dir() {
		for entry in std::fs::read_dir(path)? {
			let file = entry?;
			let longpath = file.path();
			let matched;
			if fast_scan {
				let guess = mime_guess::from_path(longpath.clone());
				let Some(guess) = guess.first() else { continue };
				let mimetype = guess.type_();
				matched = mimetype == mime::AUDIO || mimetype == mime::VIDEO;
			} else {
				let Ok(fmt) = FileFormat::from_file(longpath.clone()) else { continue; };
				let kind = fmt.kind();
				matched = kind == Kind::Audio || kind == Kind::Video;
			}
			if matched {
				let filename = longpath.file_name().unwrap().to_os_string().into_string().unwrap();
				let info = if let Some(info) = old_files.get(&filename) { info.clone() } else { ClientFile::default() };
				files.insert(filename, info);
			}
		}
		files.sort_keys();
		atomic_client_state.write().file_tabs[index] = files;
		add_duration(atomic_client_state, tab.clone());
	}
	Ok(())
}

fn scan_tabs(client_state: AtomicClientState) -> Result<(), Box<dyn std::error::Error>> {
	let len = { client_state.read().file_tabs.len() };
	let mut handles = vec![];
	for ii in 0..len {
		let client_state = client_state.clone();
		let handle = thread::spawn(move || { let _ = scan_tab(client_state, ii); });
		handles.push(handle);
	}
	for handle in handles {
		handle.join().unwrap();
	}
	Ok(())
}

pub fn scan(client_state: AtomicClientState, mode: Scanning) {
	if mode == Scanning::None {
		return;
	}
	{ client_state.write().scanning = mode; }
	match mode {
		Scanning::All => {
			log::info("Scanning all tabs...");
			let _ = scan_tabs(client_state.clone());
			log::info("Scanned all tabs");
		},
		Scanning::One(index) => {
			log::info(format!("Scanning tab {}...", index));
			let _ = scan_tab(client_state.clone(), index);
			log::info(format!("Scanned tab {}", index));
			let mut client_state = client_state.write();
			client_state.scanning = Scanning::None;
			client_state.redrawer.notify();
		},
		_ => ()
	};
}