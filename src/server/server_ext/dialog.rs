use std::{thread, time::{Duration, SystemTime}};

use rand::Rng;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

use crate::{common::base::dialog::Dialog, server::{AtomicServerState, file::play_file}};

#[derive(Clone, Default)]
pub struct ServerDialog {
	pub base: Dialog,
	pub forced: bool,
	pub play_next: usize,
}

impl ServerDialog {
	fn get_next_path(&mut self) -> &String {
		if self.base.random {
			if self.play_next == 0 {
				self.play_next = rand::thread_rng().gen_range(0..self.base.files.len());
			} else {
				self.play_next -= 1;
				let previous = self.play_next;
				while self.play_next == previous {
					self.play_next = rand::thread_rng().gen_range(0..self.base.files.len());
				}
			}
		}
		let path = &self.base.files[self.play_next];
		self.play_next = (self.play_next + 1) % self.base.files.len();
		return path;
	}

	pub fn play(&self, server_state: AtomicServerState, auto_stop: bool) {
		let mut dialog = self.clone();
		if dialog.base.files.is_empty() {
			return;
		}

		if auto_stop {
			let start = SystemTime::now();
			let mut elapsed = 0;
			while elapsed < 1000 {
				let volume = dialog.base.volume as f32 / 100.0;
				let sequential = dialog.base.sequential;
				play_file(server_state.clone(), dialog.get_next_path().clone(), volume, sequential);
				if !dialog.base.sequential {
					thread::sleep(Duration::from_secs_f32(dialog.base.delay));
				}

				// Calculate time elapsed
				let duration = SystemTime::now().duration_since(start);
				if duration.is_ok() {
					elapsed = duration.unwrap().as_millis();
				} else {
					break;
				}
			}
		} else {
			while dialog.forced || dialog.base.keys.par_iter().all(|key| { key.is_pressed() }) {
				let volume = dialog.base.volume as f32 / 100.0;
				let sequential = dialog.base.sequential;
				play_file(server_state.clone(), dialog.get_next_path().clone(), volume, sequential);
				if !dialog.base.sequential {
					thread::sleep(Duration::from_secs_f32(dialog.base.delay));
				}
			}
		}
	}
}