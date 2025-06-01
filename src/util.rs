use std::{path::Path, thread};

use ffprobe::FfProbe;
use file_format::{FileFormat, Kind};

use crate::state::{config, get_app, get_mut_app};

pub mod fs;
pub mod global_input;
pub mod pulseaudio;
pub mod threads;

pub fn ffprobe_info(path: &str) -> Option<FfProbe> {
	let result = ffprobe::ffprobe(path);
	match result {
		Ok(info) => {
			if info.streams.iter().any(|stream| stream.codec_type == Option::Some("audio".to_string())) {
				return Option::Some(info);
			} else {
				return Option::None;
			}
		},
		Err(_err) => {
			// not a media file
			return Option::None;
		}
	}
}

fn add_duration(tab: String) {
	thread::spawn(move || {
		let app = get_mut_app();
		let files = app.files.get(&tab);
		if files.is_none() {
			return;
		}
		let mut new_files = vec![];
		for (filename, _) in files.unwrap() {
			let longpath = Path::new(&tab).join(filename);
			let filepath = longpath.into_os_string().into_string().unwrap();
			let info = ffprobe_info(filepath.as_str());
			info.inspect(|info| {
				let mut duration_str = String::new();
				info.format.get_duration().inspect(|duration| {
					let millis = duration.as_millis();
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
				});
				new_files.push((filename.clone(), duration_str));
			});
		}
		app.files.insert(tab, new_files);
		notify_redraw();
	});
}

pub fn scan_tab(index: usize) -> Result<(), std::io::Error> {
	let app = get_mut_app();
	let config = config();
	if index >= config.tabs.len() {
		return Ok(());
	}
	let tab = config.tabs[index].clone();
	let mut files = vec![];
	let path = Path::new(tab.as_str());
	if path.is_dir() {
		for entry in std::fs::read_dir(path)? {
			let file = entry?;
			let longpath = file.path();
			let fmt = FileFormat::from_file(longpath.clone());
			fmt.inspect(|fmt| {
				match fmt.kind() {
					Kind::Audio|Kind::Video => {
						let filename = longpath.file_name().unwrap().to_os_string().into_string().unwrap();
						files.push((filename, String::new()));
					},
					_ => (),
				}
			});
		}
    files.sort_by(|a, b| a.0.to_lowercase().cmp(&b.0.to_lowercase()));
		app.files.insert(tab.clone(), files);
		add_duration(tab);
	}
	Ok(())
}

pub fn scan_tabs() -> Result<(), std::io::Error> {
	let app = get_mut_app();
	for ii in 0..app.config.tabs.len() {
		scan_tab(ii)?;
	}
	Ok(())
}

pub fn selected_file_path() -> String {
	let app = get_app();
	let config = config();
	if app.tab_selected() >= config.tabs.len() {
		return String::new();
	}
	let tab = config.tabs[app.tab_selected()].clone();
	let files = app.files.get(&tab);
	if files.is_none() {
		return String::new();
	}
	let files = files.unwrap();
	if app.file_selected() >= files.len() {
		return String::new();
	}
	return Path::new(&tab).join(&files[app.file_selected()].0).into_os_string().into_string().unwrap();
}

pub fn notify_redraw() {
	let app = get_app();
	let pair = app.pair.clone();
	let (lock, cvar) = &*pair;
	let mut shared = lock.lock().expect("Failed to get shared mutex");
	shared.redraw = true;
	cvar.notify_all();
	std::mem::drop(shared);
}