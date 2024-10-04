use std::{collections::HashMap, fs, path::Path, process::Command};

use crate::state::get_mut_app;

fn is_media(path: &str) -> bool {
	let output = Command::new("ffprobe").args([
		"-loglevel",
		"quiet",
		"-select_streams",
		"a",
		"-show_entries",
		"stream=codec_type",
		"-of",
		"csv=p=0",
		path.trim()
	]).output().unwrap();

	if !output.status.success() {
		return false;
	}
	let out = String::from_utf8(output.stdout).unwrap();
	return out.trim() == "audio";
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
			if is_media(filepath.as_str()) {
				files.push(filename);
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