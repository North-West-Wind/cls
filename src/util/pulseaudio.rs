use std::process::Command;

use crate::constant::APP_NAME;

pub fn load_null_sink() -> String {
	let appname = APP_NAME;
	let result = Command::new("pactl").args([
		"load-module",
		"module-null-sink",
		format!("sink_name={appname}").as_str(),
		"formats=s32le"
	]).output();

	if result.is_err() {
		return String::new();
	}

	let output = result.unwrap();
	if !output.status.success() {
		return String::new();
	}

	let result = String::from_utf8(output.stdout);
	if result.is_err() {
		return String::new();
	}
	result.unwrap()
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

pub fn loopback(sink: String) -> String {
	let result = Command::new("pactl").args([
		"load-module",
		"module-loopback",
		"source=cls.monitor",
		format!("sink={sink}").as_str(),
	]).output();

	if result.is_err() {
		return String::new();
	}

	let output = result.unwrap();
	if !output.status.success() {
		return String::new();
	}

	let result = String::from_utf8(output.stdout);
	if result.is_err() {
		return String::new();
	}
	result.unwrap()
}