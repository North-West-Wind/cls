use std::{format, path::Path, thread, vec};

use file_format::{FileFormat, Kind};
use indexmap::IndexMap;
use mime_guess::mime;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use symphonium::{ResampleQuality, SymphoniumLoader};

use crate::{client::{AtomicClientState, Scanning, client_ext::file::ClientFile}, common::{ffmpeg::read_file_ffmpeg, log}};

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

fn add_duration(atomic_client_state: AtomicClientState, tab: String) {
	let client_state = atomic_client_state.read();
	let Some((_, files)) = client_state.file_tabs.iter().find(|(key, _)| *key == tab).cloned() else { return; };
	drop(client_state);
	let mut loader = SymphoniumLoader::new();
	let mut new_files = IndexMap::new();
	for (filename, info) in files {
		let longpath = Path::new(&tab).join(filename.clone());
		let filepath = longpath.into_os_string().into_string().unwrap();
		let mut info = info.clone();

		let result = ffprobe_duration(&filepath);
		let millis: u128 = if result.is_none() {
			let result = loader.load(&filepath, None, ResampleQuality::Low, None);
			if result.is_err() {
				let result = read_file_ffmpeg(&filepath, 48000);
				if result.is_err() {
					new_files.insert(filename.clone(), info);
					continue;
				}
				result.unwrap().len() as u128 / 48
			} else {
				let audio_data = result.unwrap();
				audio_data.frames() as u128 * 1000 / audio_data.sample_rate().get() as u128
			}
		} else {
			result.unwrap()
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
	let mut client_state = atomic_client_state.write();
	if let Some((_, files)) = client_state.file_tabs.iter_mut().find(|(key, _)| *key == tab) {
		*files = new_files;
		client_state.redrawer.notify();
	}
}

fn scan_tab(atomic_client_state: AtomicClientState, index: usize) -> Result<(), Box<dyn std::error::Error>> {
	let client_state = atomic_client_state.clone();
	let client_state = client_state.read();
	let tabs = &client_state.file_tabs;
	let fast_scan = client_state.config.fast_scan;
	if index >= tabs.len() {
		return Ok(());
	}
	let (tab, old_files) = tabs[index].clone();
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
		atomic_client_state.write().file_tabs[index] = (tab.clone(), files);
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