use std::process::Command;

use crate::{constant::APP_NAME, state::get_app};

pub fn load_null_sink() -> Result<String, Box<dyn std::error::Error>> {
	let appname = APP_NAME;
	let output = Command::new("pactl").args([
		"load-module",
		"module-null-sink",
		format!("sink_name={appname}").as_str()
	]).output()?;

	if !output.status.success() {
		return Ok(String::new());
	}

	let index = String::from_utf8(output.stdout)?;
	Ok(index)
}

pub fn unload_null_sink() -> Result<(), Box<dyn std::error::Error>> {
	let app = get_app();
	if !app.module_num.is_empty() {
		let output = Command::new("pactl").args([
			"unload-module",
			app.module_num.trim()
		]).output()?;
		
		if !output.status.success() {
			println!("Failed to unload module");
		}
	}

	Ok(())
}