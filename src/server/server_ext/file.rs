use std::{format, io::Read, path::Path, process::{Command, Stdio}, sync::Arc, thread::{self, JoinHandle}, time::Duration, vec};

use parking_lot::Mutex;
use ringbuf::{HeapProd, HeapRb, traits::{Producer, Split}};
use uuid::Uuid;

use crate::{common::{base::file::SaveableFile, constant::ENDIANESS, log}, server::ServerState};

pub struct ServerFile {
	pub base: SaveableFile,
	lock: Arc<Mutex<()>>,
	path: String
}

impl ServerFile {
	pub fn new(path: String, server_state: &ServerState) -> Self {
		let lock = if server_state.config.playlist_mode {
			server_state.playlist_lock.clone()
		} else {
			Arc::new(Mutex::new(()))
		};

		Self::new_with_lock(path, lock, server_state)
	}

	pub fn new_with_lock(path: String, lock: Arc<Mutex<()>>, server_state: &ServerState) -> Self {
		let pathed = Path::new(&path);
		let parent = pathed.parent().unwrap().to_str().unwrap().to_string();
		let name = pathed.file_name().unwrap().to_os_string().into_string().unwrap();
		let base = match server_state.config.files.get(&parent) {
			Some(map) => {
				match map.get(&name) {
					Some(entry) => entry.clone(),
					None => SaveableFile::default(),
				}
			},
			None => SaveableFile::default()
		};

		Self { base, lock, path }
	}

	pub fn play(&self, server_state: &mut ServerState) -> Option<JoinHandle<()>> {
		let uuid = Uuid::new_v4();
		let (sample_rate, prod) = {
			let sample_rate = server_state.sample_rate as usize;
			let rb = HeapRb::<f32>::new(sample_rate / 16);
			let (prod, cons) = rb.split();
			server_state.audio_data.insert(uuid, Arc::new(Mutex::new(cons)));
			(sample_rate, prod)
		};

		match self.play_with_ffmpeg(sample_rate, prod, self.lock.clone()) {
			Ok(thread) => Some(thread),
			Err(err) => {
				log::error(format!("Failed to play file with ffmpeg: {:?}", err));
				server_state.audio_data.remove(&uuid);
				None
			}
		}
	}

	fn play_with_ffmpeg(&self, sample_rate: usize, mut prod: HeapProd<f32>, lock: Arc<Mutex<()>>) -> Result<JoinHandle<()>, Box<dyn std::error::Error>> {
		let mut result = Command::new("ffmpeg").args([
			"-loglevel", "-8",
			"-i", &self.path,
			"-f", format!("f32{}", ENDIANESS).as_str(),
			"-ac", "2",
			"-ar", sample_rate.to_string().as_str(),
			"-"
		]).stdout(Stdio::piped()).spawn()?;
		let mut stdout = result.stdout.take().unwrap();
		Ok(thread::spawn(move || {
			let _locked = lock.lock();
			let mut buf_size = sample_rate * 4 / 16;
			while buf_size % 4 != 0 {
				buf_size += 1;
			}
			let mut buf = vec![0u8; buf_size];
			loop {
				match stdout.read(&mut buf) {
					Ok(read) => {
						if read == 0 {
							break;
						}

						let read = read / 4;
						let buf: &[f32] = bytemuck::cast_slice(&buf);
						let mut offset = prod.push_slice(&buf[..read]);
						while offset < read {
							thread::sleep(Duration::from_millis(10));
							offset += prod.push_slice(&buf[offset..read]);
						}
					},
					Err(err) => {
						log::error(format!("ffmpeg error: {:?}", err));
						break;
					}
				}
			}
		}))
	}
}