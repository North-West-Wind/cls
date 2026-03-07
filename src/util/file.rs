use std::{collections::HashMap, num::NonZero, path::Path, sync::{Arc, Condvar, LazyLock, Mutex, MutexGuard}, thread, time::Duration};

use symphonium::{DecodedAudio, ResampleQuality, SymphoniumLoader};
use uuid::Uuid;

use crate::{component::block::log, state::{acquire, notify_redraw}};

pub struct PlayableFile {
	pub audio_data: DecodedAudio,
	pub position: usize,
	pub volume: f32,
	pub finished: Arc<(Mutex<()>, Condvar)>,
}

static PLAYING_FILES: LazyLock<Mutex<HashMap<Uuid, PlayableFile>>> = LazyLock::new(|| { Mutex::new(HashMap::new()) });
static SYMPHONIUM_LOADER: LazyLock<Mutex<SymphoniumLoader>> = LazyLock::new(|| { Mutex::new(SymphoniumLoader::new()) });

pub fn acquire_playing_files() -> MutexGuard<'static, HashMap<Uuid, PlayableFile>> {
	PLAYING_FILES.lock().unwrap()
}

pub fn play_file_auto_volume(path: &String, lock: Arc<Mutex<()>>) {
	let path = path.clone();
	thread::spawn(move || {
		let app = acquire();
		let pathed = Path::new(&path);
		let parent = pathed.parent().unwrap().to_str().unwrap().to_string();
		let name = pathed.file_name().unwrap().to_os_string().into_string().unwrap();
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
		play_file(&path, volume, lock);
	});
}

pub fn play_file(path: &String, volume: f32, lock: Arc<Mutex<()>>) {
	let string = path.trim().to_string();
	thread::spawn(move || {
		let _locked = lock.lock().expect("Failed to lock while playing file");

		let uuid = Uuid::new_v4();
		let mut app = acquire();
		if app.edit {
			app.playing_file.insert(uuid, "Edit-only mode!".to_string());
			notify_redraw();
			drop(app);
			thread::sleep(Duration::from_secs(1));
			acquire().playing_file.remove(&uuid);
			notify_redraw();
			return;
		}
		drop(app);

		let mut loader = SYMPHONIUM_LOADER.lock().unwrap();

		let result = loader.load(&string, NonZero::new(48000), ResampleQuality::Low, None);
		if result.is_err() {
			log::error(format!("File {} cannot be decoded", string).as_str());
			log::error(format!("{:?}", result.unwrap_err()).as_str());
			return;
		}
		let audio_data = result.unwrap();
		drop(loader);

		let finished = Arc::new((Mutex::new(()), Condvar::new()));
		acquire_playing_files().insert(uuid, PlayableFile { audio_data, position: 0, volume, finished: finished.clone() });
		let mut app = acquire();
		app.playing_file.insert(uuid, string.to_string());
		drop(app);
		notify_redraw();

		let (lock, cvar) = &*finished;
		drop(cvar.wait(lock.lock().unwrap()).unwrap());
	});
}

pub fn stop_all() {
	// Defer to avoid deadlock
	thread::spawn(move || {
		acquire_playing_files().clear();
		acquire().playing_file.clear();
	});
}