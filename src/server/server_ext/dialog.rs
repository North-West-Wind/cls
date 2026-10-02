use std::{sync::{Arc, atomic::{AtomicBool, Ordering}}, thread::{self, JoinHandle}, time::Duration};

use parking_lot::Mutex;
use rand::Rng;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

use crate::{common::base::dialog::Dialog, server::{AtomicServerState, server_ext::file::ServerFile}};

#[derive(Clone, Default)]
pub struct ServerDialog {
	pub base: Dialog,
	pub playing: Arc<AtomicBool>,
}

impl ServerDialog {
	pub fn play(&self, server_state: AtomicServerState) -> Option<JoinHandle<()>> {
		if self.base.files.is_empty() {
			return None;
		}

		let playing = self.playing.clone();
		let keys = self.base.keys.clone();
		let files = {
			let server_state = server_state.read();
			self.base.files.iter().map(|path| {
				let mut file = ServerFile::new_with_lock(path.clone(), Arc::new(Mutex::new(())), &server_state);
				file.base.volume = self.base.volume;
				file
			}).collect::<Vec<_>>()
		};
		let delay = self.base.delay;
		let random = self.base.random;
		let sequential = self.base.sequential;

		Some(thread::spawn(move || {
			let mut play_next = 0;
			while playing.load(Ordering::Relaxed) || keys.par_iter().all(|key| { key.is_pressed() }) {
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

				let thread = file.play(&mut server_state.write());
				if !sequential {
					thread::sleep(Duration::from_secs_f32(delay));
				} else if let Some(thread) = thread {
					let _ = thread.join();
				}
			}
		}))
	}
}