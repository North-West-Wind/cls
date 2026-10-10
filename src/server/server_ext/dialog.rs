use std::{sync::{Arc, atomic::{AtomicBool, Ordering}}, thread::{self, JoinHandle}, time::Duration};

use parking_lot::RwLock;
use rand::Rng;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

use crate::{common::base::dialog::Dialog, server::{AudioData, server_ext::file::ServerFile}};

#[derive(Clone, Default)]
pub struct ServerDialog {
	pub base: Dialog,
	pub active: Arc<AtomicBool>,
}

impl From<Dialog> for ServerDialog {
	fn from(base: Dialog) -> Self {
		Self {
			base,
			active: Arc::new(AtomicBool::new(false)),
		}
	}
}

impl ServerDialog {
	pub fn play(&self, sample_rate: u32, audio_data: Arc<RwLock<AudioData>>) -> Option<JoinHandle<()>> {
		if self.base.files.is_empty() {
			return None;
		}

		let active = self.active.clone();
		let files = self.base.files.par_iter().map(|path| ServerFile::new(path.clone(), self.base.volume as f32 / 100.0, false)).collect::<Vec<_>>();
		let delay = self.base.delay;
		let random = self.base.random;
		let sequential = self.base.sequential;

		Some(thread::spawn(move || {
			let mut play_next = 0;
			while active.load(Ordering::Relaxed) {
				if random {
					if play_next == 0 {
						play_next = rand::thread_rng().gen_range(0..files.len());
					} else {
						play_next -= 1;
						let previous = play_next;
						while play_next == previous {
							play_next = rand::thread_rng().gen_range(0..files.len());
						}
					}
				}
				let file = &files[play_next];
				play_next = (play_next + 1) % files.len();

				let thread = file.play(sample_rate, &mut audio_data.write());
				if !sequential {
					thread::sleep(Duration::from_secs_f32(delay));
				} else if let Some(thread) = thread {
					let _ = thread.join();
				}
			}
		}))
	}
}