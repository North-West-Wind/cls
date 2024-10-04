use std::process::Command;

use libpulse_binding::volume::{ChannelVolumes, Volume};
use pulsectl::controllers::{DeviceControl, SinkController};

use crate::{constant::APP_NAME, state::{get_app, get_mut_app}};


pub fn load_sink_controller() -> Result<SinkController, Box<dyn std::error::Error>> {
	Ok(SinkController::create()?)
}

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

pub fn set_volume_percentage(percentage: u32) {
	let app = get_mut_app();
	if app.sink_controller.is_none() {
		return;
	}
	let controller = app.sink_controller.as_mut().unwrap();
	let device = controller.get_device_by_name(APP_NAME);
	if device.is_err() {
		return;
	}
	let mut device = device.unwrap();
	controller.set_device_volume_by_name(APP_NAME, device.volume.set(device.volume.len(), Volume(Volume::NORMAL.0 * percentage / 100)));
}