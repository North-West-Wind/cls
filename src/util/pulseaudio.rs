use std::{path::Path, process::{Command, Stdio}, thread, time::Duration};

use nix::{sys::signal::{self, Signal}, unistd::Pid};
use uuid::Uuid;

use crate::{constant::APP_NAME, state::{acquire, acquire_playlist_lock, notify_redraw}, util::ffprobe_info};

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

pub fn play_file(path: &String) {
	let string = path.trim().to_string();
	thread::spawn(move || {
		let uuid = Uuid::new_v4();
		let mut app = acquire();
		if app.edit {
			app.playing_file.insert(uuid, (0, "Edit-only mode!".to_string()));
			notify_redraw();
			drop(app);
			thread::sleep(Duration::from_secs(1));
			acquire().playing_file.remove(&uuid);
			notify_redraw();
			return;
		}
		drop(app);

		let info = ffprobe_info(&string);
		info.inspect(|info| {
			info.streams.iter()
				.find(|stream| stream.codec_type == Option::Some("audio".to_string()))
				.inspect(|stream| {
					let mut app = acquire();
					let using_semaphore = app.config.playlist_mode;
					app.playing_file.insert(uuid, (0, string.to_string()));
					drop(app);
					notify_redraw();
					let playlist_lock = if using_semaphore {
						Some(acquire_playlist_lock())
					} else {
						None
					};

					let ffmpeg_child = Command::new("ffmpeg").args([
						"-loglevel",
						"-8",
						"-i",
						&string,
						"-f",
						"s16le",
						"-"
					]).stdout(Stdio::piped()).spawn().expect("Failed to spawn ffmpeg process");

					let mut app = acquire();
					let path = Path::new(&string);
					let parent = path.parent().unwrap().to_str().unwrap().to_string();
					let name = path.file_name().unwrap().to_os_string().into_string().unwrap();
					let volume: u16 = match app.config.files.get(&parent) {
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

					app.playing_file.insert(uuid, (pacat_child.id(), string.to_string()));
					drop(app);

					let _ = pacat_child.wait();
					if playlist_lock.is_some() {
						drop(playlist_lock.unwrap());
					}
				});
		});
		let mut app = acquire();
		app.playing_file.remove(&uuid);
		notify_redraw();
	});
}

pub fn stop_all() {
	// Defer to avoid deadlock
	thread::spawn(move || {
		let mut app = acquire();
		for (id, _file) in app.playing_file.values() {
			signal::kill(Pid::from_raw(*id as i32), Signal::SIGTERM).ok();
		}
		app.playing_file.clear();
	});
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