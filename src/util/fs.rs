use std::path::Path;

pub fn separate_parent_file(str: String) -> (String, String) {
	let path = Path::new(&str);
	let parent = path.parent().unwrap().to_str().unwrap().to_string();
	let name = path.file_name().unwrap().to_str().unwrap().to_string();
	(parent, name)
}