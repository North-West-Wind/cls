use std::{collections::HashMap, io::{Error, Read}, num::NonZero, path::Path, process::{Command, Stdio}, sync::{Arc, Condvar, LazyLock, Mutex, MutexGuard}, thread, time::Duration};

use symphonium::{ResampleQuality, SymphoniumLoader};
use uuid::Uuid;

use crate::{component::block::log, constant::ENDIANESS, state::{acquire, notify_redraw}};

pub fn parent_file(str: String) -> (String, String) {
	let path = Path::new(&str);
	let parent = path.parent().unwrap().to_str().unwrap().to_string();
	let name = path.file_name().unwrap().to_str().unwrap().to_string();
	(parent, name)
}

pub struct PlayableFile {
	pub data: Vec<f32>,
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
		let result = loader.load_f32(&string, NonZero::new(48000), ResampleQuality::Low, None);
		drop(loader);
		let interleaved = if result.is_err() {
			log::error(format!("File {} cannot be decoded with symphonium", string).as_str());
			log::error(format!("{:?}", result.unwrap_err()).as_str());

			let result = read_file_ffmpeg(&string);
			if result.is_err() {
				log::error(format!("File {} cannot be decoded with ffmpeg", string).as_str());
				log::error(format!("{:?}", result.unwrap_err()).as_str());
				return;
			}
			result.unwrap()
		} else {
			let audio_data = result.unwrap();
			if audio_data.channels() == 1 {
				audio_data.data[0].iter().zip(audio_data.data[0].iter()).flat_map(|(a, b)| [*a, *b]).collect()
			} else if audio_data.channels() > 2 {
				audio_data.data[0].iter().zip(audio_data.data[1].iter()).flat_map(|(a, b)| [*a, *b]).collect()
			} else {
				audio_data.as_interleaved()
			}
		};

		let finished = Arc::new((Mutex::new(()), Condvar::new()));
		acquire_playing_files().insert(uuid, PlayableFile { data: interleaved, position: 0, volume, finished: finished.clone() });
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

pub fn read_file_ffmpeg(path: &str) -> Result<Vec<f32>, Error> {
	let result = Command::new("ffmpeg").args([
		"-loglevel", "-8",
		"-i", path,
		"-f", format!("f32{}", ENDIANESS).as_str(),
		"-ac", "2",
		"-ar", "48000",
		"-"
	]).stdout(Stdio::piped()).spawn();
	if result.is_err() {
		return Err(result.unwrap_err());
	}
	let mut buf = vec![];
	let _ = result.unwrap().stdout.unwrap().read_to_end(&mut buf);
	Ok(buf.chunks(4).map(|chunk| f32::from_le_bytes(chunk.try_into().unwrap())).collect())
}