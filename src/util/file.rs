use std::{collections::HashMap, num::NonZero, path::Path, sync::{Arc, Condvar, LazyLock, Mutex, MutexGuard}, thread, time::Duration};

use symphonium::{DecodedAudioF32, ResampleQuality, SymphoniumLoader};
use uuid::Uuid;

use crate::{state::{acquire, acquire_playlist_lock, notify_redraw}, util::ffprobe_info};

pub struct PlayableFile {
	pub head: usize,
	pub data: DecodedAudioF32,
	pub volume: f32,
	pub signal: Arc<(Mutex<bool>, Condvar)>
}

static PLAYING_FILES: LazyLock<Mutex<HashMap<Uuid, PlayableFile>>> = LazyLock::new(|| { Mutex::new(HashMap::new()) });

pub fn acquire_playing_files() -> MutexGuard<'static, HashMap<Uuid, PlayableFile>> {
	PLAYING_FILES.lock().unwrap()
}

pub fn play_file(path: &String) {
	let string = path.trim().to_string();
	thread::spawn(move || {
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

		let info = ffprobe_info(&string);
		info.inspect(|info| {
			info.streams.iter()
				.find(|stream| stream.codec_type == Option::Some("audio".to_string()))
				.inspect(|_| {
					let mut app = acquire();
					let using_semaphore = app.config.playlist_mode;
					app.playing_file.insert(uuid, string.to_string());
					drop(app);
					notify_redraw();
					let playlist_lock = if using_semaphore {
						Some(acquire_playlist_lock())
					} else {
						None
					};

					let mut app = acquire();
					let mut loader = SymphoniumLoader::new();
					let audio_data = loader.load_f32(
						&string,
						Option::expect(Some(NonZero::new(48000)), "Failed to create sample rate"),
						ResampleQuality::High,
						None).unwrap();

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
					let signal = Arc::new((Mutex::new(false), Condvar::new()));
					acquire_playing_files().insert(uuid, PlayableFile {
						head: 0,
						data: audio_data,
						volume,
						signal: signal.clone()
					});
					app.playing_file.insert(uuid, string.to_string());
					drop(app);

					let (lock, cvar) = &*signal;
					let mut ended = lock.lock().unwrap();
					while !*ended {
				    ended = cvar.wait(ended).unwrap();
					}
					if playlist_lock.is_some() {
						drop(playlist_lock.unwrap());
					}
				});
		});
	});
}

pub fn stop_all() {
	// Defer to avoid deadlock
	thread::spawn(move || {
		let mut playing_files = acquire_playing_files();
		playing_files.iter().for_each(|(_uuid, playable)| {
	    let (lock, cvar) = &*playable.signal;
	    let mut ended = lock.lock().unwrap();
	    *ended = true;
	    cvar.notify_one();
		});
		playing_files.clear();
		acquire().playing_file.clear();
	});
}