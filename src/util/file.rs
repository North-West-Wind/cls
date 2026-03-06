use std::{collections::HashMap, io::BufReader, path::Path, process::{ChildStdout, Command, Stdio}, sync::{Arc, LazyLock, Mutex, MutexGuard}, thread, time::Duration};

use nix::{sys::signal::{self, Signal}, unistd::Pid};
use uuid::Uuid;

use crate::state::{acquire, notify_redraw};

pub struct PlayableFile {
	pub reader: BufReader<ChildStdout>,
	pub volume: f32,
}

static PLAYING_FILES: LazyLock<Mutex<HashMap<Uuid, PlayableFile>>> = LazyLock::new(|| { Mutex::new(HashMap::new()) });

pub fn acquire_playing_files() -> MutexGuard<'static, HashMap<Uuid, PlayableFile>> {
	PLAYING_FILES.lock().unwrap()
}

pub fn play_file(path: &String, lock: Arc<Mutex<()>>) {
	let string = path.trim().to_string();
	thread::spawn(move || {
		let _locked = lock.lock().expect("Failed to lock while playing file");

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
		let path = Path::new(&string);
		let parent = path.parent().unwrap().to_str().unwrap().to_string();
		let name = path.file_name().unwrap().to_os_string().into_string().unwrap();
		let volume: f32 = match app.config.files.get(&parent) {
			Some(map) => {
				match map.get(&name) {
					Some(entry) => (entry.volume as f32) / 100.0,
					None => 1.0,
				}
			},
			None => 1.0
		};
		drop(app);

		let mut ffmpeg_child = Command::new("ffmpeg").args([
			"-loglevel", "-8",
			"-i", &string,
			"-f", "f32le",
			"-ac", "2",
			"-ar", "48000",
			"-"
		]).stdout(Stdio::piped()).spawn().expect("Failed to spawn ffmpeg process");

		acquire_playing_files().insert(uuid, PlayableFile { reader: BufReader::new(ffmpeg_child.stdout.take().unwrap()), volume });
		let mut app = acquire();
		app.playing_file.insert(uuid, (ffmpeg_child.id(), string.to_string()));
		drop(app);
		notify_redraw();

		let _ = ffmpeg_child.wait();
	});
}

pub fn stop_all() {
	// Defer to avoid deadlock
	thread::spawn(move || {
		acquire_playing_files().clear();
		let mut app = acquire();
		for (id, _file) in app.playing_file.values() {
			signal::kill(Pid::from_raw(*id as i32), Signal::SIGTERM).ok();
		}
		app.playing_file.clear();
	});
}