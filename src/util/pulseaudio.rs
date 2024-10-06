use std::{process::{Command, Stdio}, thread};

use libpulse_binding::volume::Volume;
use pulsectl::controllers::{DeviceControl, SinkController};
use uuid::Uuid;

use crate::{constant::APP_NAME, state::{get_app, get_mut_app}, util::ffprobe_info};

use super::notify_redraw;


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

pub fn play_file(path: &str) {
	let string = path.trim().to_string();
	thread::spawn(move || {
		let uuid = Uuid::new_v4();
		let app = get_mut_app();
		app.playing.as_mut().unwrap().insert(uuid, string.clone());
		notify_redraw();

		let info = ffprobe_info(string.as_str());
		if info.is_some() {
			let info = info.unwrap();
			let stream = info.streams.iter().find(|stream| stream.codec_type == Option::Some("audio".to_string()));
			if stream.is_some() {
				let stream = stream.unwrap();

				let ffmpeg_child = Command::new("ffmpeg").args([
					"-loglevel",
					"-8",
					"-i",
					string.as_str(),
					"-f",
					"s16le",
					"-"
				]).stdout(Stdio::piped()).spawn().unwrap();

				let volume: u16;
				if app.config.file_volume.is_some() {
					volume = (app.config.file_volume.as_ref().unwrap().get(&string).unwrap_or(&100) * 65535 / 100) as u16;
				} else {
					volume = 65535;
				}
		
				let _ = Command::new("pacat").args([
					"-d",
					APP_NAME,
					format!("--channels={}", stream.channels.unwrap_or(2)).as_str(),
					format!("--rate={}", stream.sample_rate.clone().unwrap()).as_str(),
					format!("--volume={}", volume).as_str(),
				]).stdin(Stdio::from(ffmpeg_child.stdout.unwrap())).stdout(Stdio::piped()).spawn().unwrap().wait();
			}
		}
		app.playing.as_mut().unwrap().remove(&uuid);
		notify_redraw();
	});
}