use std::{format, num::NonZero, path::Path, sync::Arc, time::SystemTime};

use parking_lot::{Condvar, Mutex};
use rayon::iter::{IndexedParallelIterator, IntoParallelRefIterator, ParallelIterator};
use symphonium::ResampleQuality;
use uuid::Uuid;

use crate::{common::{ffmpeg::read_file_ffmpeg, log}, server::AtomicServerState};

pub struct PlayableFile {
	pub data: Vec<f32>,
	pub position: usize,
	pub volume: f32,
	pub finished: Arc<(Mutex<()>, Condvar)>,
}

pub fn play_file_auto_volume(server_state: AtomicServerState, path: String, block: bool) -> Result<(), Box<dyn std::error::Error>> {
	let path = path.clone();
	let pathed = Path::new(&path);
	let parent = pathed.parent().unwrap().to_str().unwrap().to_string();
	let name = pathed.file_name().unwrap().to_os_string().into_string().unwrap();
	let volume: f32 = match server_state.read().config.files.get(&parent) {
		Some(map) => {
			match map.get(&name) {
				Some(entry) => (entry.volume as f32) / 100.0,
				None => 1.0,
			}
		},
		None => 1.0
	};
	play_file(server_state, path, volume, block);
	Ok(())
}

pub fn play_file(server_state: AtomicServerState, path: String, volume: f32, block: bool) {
	let lock = {
		let server_state = server_state.read();
		if server_state.config.playlist_mode {
			server_state.playlist_lock.clone()
		} else {
			Arc::new(Mutex::new(()))
		}
	};

	let string = path.trim().to_string();
	let _locked = lock.lock();
	let mut server_state = server_state.write();

	let uuid = Uuid::new_v4();
	let sample_rate = server_state.sample_rate;

	let interleaved = if server_state.file_cache.contains_key(&string) {
		let (data, last_accessed) = server_state.file_cache.get_mut(&string).unwrap();
		*last_accessed = SystemTime::now();
		let data = data.clone();
		data
	} else {
		let data = match server_state.symphonium_loader.lock().load_f32(&string, NonZero::new(sample_rate), ResampleQuality::Low, None) {
			Err(err) => {
				log::warn(format!("File {} cannot be decoded with symphonium: {:?}", string, err));
				match read_file_ffmpeg(&string, sample_rate) {
					Err(err) => {
						log::error(format!("File {} cannot be decoded with ffmpeg {:?}", string, err));
						return;
					},
					Ok(data) => data
				}
			},
			Ok(audio_data) => {
				if audio_data.channels() == 1 {
					audio_data.data[0].par_iter().zip(audio_data.data[0].par_iter()).flat_map(|(a, b)| [*a, *b]).collect()
				} else if audio_data.channels() > 2 {
					audio_data.data[0].par_iter().zip(audio_data.data[1].par_iter()).flat_map(|(a, b)| [*a, *b]).collect()
				} else {
					audio_data.as_interleaved()
				}
			}
		};
		server_state.file_cache.insert(string.clone(), (data.clone(), SystemTime::now()));
		data
	};

	let finished = Arc::new((Mutex::new(()), Condvar::new()));
	server_state.playable_files.insert(uuid, PlayableFile { data: interleaved, position: 0, volume, finished: finished.clone() });
	drop(server_state);

	if block {
		let &(ref lock, ref cvar) = &*finished;
		let mut shared = lock.lock();
		cvar.wait(&mut shared);
	}
}

pub fn stop_all(server_state: AtomicServerState) {
	server_state.write().playable_files.clear();
}