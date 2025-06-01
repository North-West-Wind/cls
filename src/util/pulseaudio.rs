use std::{path::Path, process::{Command, Stdio}, thread, time::Duration};

use nix::{sys::signal::{self, Signal}, unistd::Pid};
use uuid::Uuid;

use crate::{constant::APP_NAME, state::{config, get_mut_app}, util::ffprobe_info};

use super::notify_redraw;

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
	]).spawn();
}

pub fn play_file(path: &str) {
	let string = path.trim().to_string();
	thread::spawn(move || {
		let uuid = Uuid::new_v4();
		let app = get_mut_app();
		let config = config();

		if app.edit {
			app.playing_file.insert(uuid, "Edit-only mode!".to_string());
			notify_redraw();
			thread::sleep(Duration::from_secs(1));
			app.playing_file.remove(&uuid);
			notify_redraw();
			return;
		}

		let info = ffprobe_info(string.as_str());
		info.inspect(|info| {
			info.streams.iter()
				.find(|stream| stream.codec_type == Option::Some("audio".to_string()))
				.inspect(|stream| {
					let using_semaphore = config.playlist_mode;
					app.playing_file.insert(uuid, string.clone());
					notify_redraw();
					if using_semaphore {
						app.playing_semaphore.acquire();
					}

					let ffmpeg_child = Command::new("ffmpeg").args([
						"-loglevel",
						"-8",
						"-i",
						string.as_str(),
						"-f",
						"s16le",
						"-"
					]).stdout(Stdio::piped()).spawn().expect("Failed to spawn ffmpeg process");

					let path = Path::new(&string);
					let parent = path.parent().unwrap().to_str().unwrap().to_string();
					let name = path.file_name().unwrap().to_os_string().into_string().unwrap();
					let volume: u16 = match config.files.get(&parent) {
						Some(map) => {
							match map.get(&name) {
								Some(entry) => (entry.volume * 65535 / 100) as u16,
								None => 65535,
							}
						},
						None => 65535
					};
			
					let mut pacat_child = Command::new("pacat").args([
						"-d",
						APP_NAME,
						format!("--channels={}", stream.channels.unwrap_or(2)).as_str(),
						format!("--rate={}", stream.sample_rate.clone().unwrap()).as_str(),
						format!("--volume={}", volume).as_str(),
					])
						.stdin(Stdio::from(ffmpeg_child.stdout.expect("Failed to obtain ffmpeg stdout")))
						.stdout(Stdio::piped()).spawn().expect("Failed to spawn pacat process");

					app.playing_process.insert(uuid, pacat_child.id());

					let _ = pacat_child.wait();
					if using_semaphore {
						app.playing_semaphore.release();
					}
				});
		});
		app.playing_file.remove(&uuid);
		app.playing_process.remove(&uuid);
		notify_redraw();
	});
}

pub fn stop_all() {
	let app = get_mut_app();
	for id in app.playing_process.values() {
		signal::kill(Pid::from_raw(*id as i32), Signal::SIGTERM);
	}
	app.playing_process.clear();
	app.playing_file.clear();
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