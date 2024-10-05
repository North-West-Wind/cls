use std::{process::Command, thread};

use libpulse_binding::volume::{ChannelVolumes, Volume};
use pulsectl::controllers::{DeviceControl, SinkController};

use crate::{constant::APP_NAME, state::{get_app, get_mut_app, CondvarPair}};


pub fn load_sink_controller() -> Result<SinkController, Box<dyn std::error::Error>> {
	Ok(SinkController::create()?)
}

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

pub fn play_file(pair: CondvarPair, path: &str) {
	let string = path.trim().to_string();
	thread::spawn(move || {
		let app = get_mut_app();

		let (lock, cvar) = &*pair;
		let mut shared = lock.lock().unwrap();
		(*shared).redraw = true;
		app.playing.push_back(string.clone());
		cvar.notify_all();
		std::mem::drop(shared);

		let _ = Command::new("paplay").args([
			"-d",
			APP_NAME,
			string.as_str()
		]).output();

		let (lock, cvar) = &*pair;
		let mut shared = lock.lock().unwrap();
		(*shared).redraw = true;
		app.playing.pop_front();
		cvar.notify_all();
	});
}