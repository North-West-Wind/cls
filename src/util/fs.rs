use std::{env, fs, path::Path};

pub fn separate_parent_file(str: String) -> (String, String) {
	let path = Path::new(&str);
	let parent = path.parent().unwrap().to_str().unwrap().to_string();
	let name = path.file_name().unwrap().to_str().unwrap().to_string();
	(parent, name)
}

pub fn command_exists(str: &str) -> bool {
	if let Ok(path) = env::var("PATH") {
		for p in path.split(":") {
			let p_str = format!("{}/{}", p, str);
			if fs::metadata(p_str).is_ok() {
				return true;
			}
		}
	}
	false
}