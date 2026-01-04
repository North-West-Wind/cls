use std::process::Command;

use crate::constant::APP_NAME;

pub fn load_null_sink() -> Result<String, Box<dyn std::error::Error>> {
	let appname = APP_NAME;
	let output = Command::new("pactl").args([
		"load-module",
		"module-null-sink",
		format!("sink_name={appname}").as_str(),
		"formats=s32le"
	]).output()?;

	if !output.status.success() {
		return Ok(String::new());
	}

	let index = String::from_utf8(output.stdout)?;
	Ok(index)
}

pub fn unload_module(module: &str) -> Result<(), Box<dyn std::error::Error>> {
	if module.trim().is_empty() {
		return Ok(());
	}
	let output = Command::new("pactl").args([
		"unload-module",
		module.trim()
	]).output()?;
	
	if !output.status.success() {
		println!("Failed to unload module {}", module);
	}
	Ok(())
}

pub fn set_volume_percentage(percentage: u32) {
	Command::new("pactl").args([
		"set-sink-volume",
		APP_NAME,
		format!("{}%", percentage).as_str(),
	]).spawn().ok();
}

pub fn loopback(sink: String) -> Result<String, Box<dyn std::error::Error>> {
	let output = Command::new("pactl").args([
		"load-module",
		"module-loopback",
		"source=cls.monitor",
		format!("sink={sink}").as_str(),
	]).output()?;

	if !output.status.success() {
		return Ok(String::new());
	}

	let index = String::from_utf8(output.stdout)?;
	Ok(index)
}