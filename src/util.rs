use std::{collections::HashMap, fs, path::Path, time::Duration};

use crate::state::get_mut_app;

fn ffprobe_info(path: &str) -> Option<Option<Duration>> {
	let result = ffprobe::ffprobe(path);
	match result {
		Ok(info) => {
			if info.streams.iter().filter(|stream| stream.codec_type == Option::Some("audio".to_string())).count() > 0 {
				return Option::Some(info.format.get_duration());
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

pub fn scan_tab(index: usize) -> Result<(), std::io::Error> {
	let app = get_mut_app();
	if index >= app.config.tabs.len() {
		return Ok(());
	}
	let tab = app.config.tabs[index].clone();
	let mut files = vec![];
	let path = Path::new(tab.as_str());
	if path.is_dir() {
		for entry in fs::read_dir(path)? {
			let file = entry?;
			let longpath = file.path();
			let filename = longpath.file_name().unwrap().to_os_string().into_string().unwrap();
			let filepath = longpath.into_os_string().into_string().unwrap();
			let info = ffprobe_info(filepath.as_str());
			if info.is_some() {
				let duration = info.unwrap();
				let duration_str: String;
				if duration.is_some() {
					duration_str = humantime::format_duration(duration.unwrap()).to_string();
				} else {
					duration_str = String::new();
				}
				files.push((filename, duration_str));
			}
		}
		app.files.as_mut().unwrap().insert(tab, files);
	}
	Ok(())
}

pub fn scan_tabs() -> Result<(), std::io::Error> {
	let app = get_mut_app();
	app.files = Option::Some(HashMap::default());
	for ii in 0..app.config.tabs.len() {
		scan_tab(ii)?;
	}
	Ok(())
}