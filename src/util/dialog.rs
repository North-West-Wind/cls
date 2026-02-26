use std::{collections::HashSet, sync::{Arc, Mutex}, thread, time::{Duration, SystemTime}};

use mki::Keyboard;
use rand::Rng;

use crate::{config::DialogEntry, state::{acquire, notify_redraw}, util::{file::play_file, global_input::keyboard_to_string}};

#[derive(Clone)]
pub struct Dialog {
	pub label: String,
	pub id: Option<u32>,
	pub keys: Vec<Keyboard>,
	pub files: Vec<String>,
	pub delay: f32,
	pub random: bool,
	pub play_next: usize,
	pub playing: Arc<Mutex<(bool, bool)>>
}

impl Default for Dialog {
	fn default() -> Self {
		Self {
			label: "New Dialog".to_string(),
			id: Option::None,
			keys: vec![],
			files: vec![],
			delay: 0.2,
			random: true,
			play_next: 0,
			playing: Arc::new(Mutex::new((false, false)))
		}
	}
}

impl Dialog {
	pub fn to_entry(&self) -> DialogEntry {
		DialogEntry {
			label: self.label.clone(),
			id: self.id,
			keys: self.keys.iter().map(|key| { keyboard_to_string(*key) }).collect::<HashSet<String>>(),
			files: self.files.clone(),
			delay: self.delay,
			random: self.random,
		}
	}

	fn get_next_path(&mut self) -> &String {
		if self.random {
			if self.play_next == 0 {
				self.play_next = rand::thread_rng().gen_range(0..self.files.len());
			} else {
				self.play_next -= 1;
				let previous = self.play_next;
				while self.play_next == previous {
					self.play_next = rand::thread_rng().gen_range(0..self.files.len());
				}
			}
		}
		let path = &self.files[self.play_next];
		self.play_next = (self.play_next + 1) % self.files.len();
		return path;
	}

	pub fn play(&self, auto_stop: bool) {
		let mut dialog = self.clone();
		thread::spawn(move || {
			if acquire().edit {
				return;
			}

			if dialog.files.is_empty() {
				return;
			}

			let mut playing = dialog.playing.lock().expect("Failed to check if dialog is playing");
			if playing.0 {
				return;
			}
			playing.0 = true;
			drop(playing);

			if auto_stop {
				let start = SystemTime::now();
				let mut elapsed = 0;
				while elapsed < 1000 {
					play_file(dialog.get_next_path());
					thread::sleep(Duration::from_secs_f32(dialog.delay));

					// Calculate time elapsed
					let duration = SystemTime::now().duration_since(start);
					if duration.is_ok() {
						elapsed = duration.unwrap().as_millis();
					} else {
						break;
					}
				}
			} else {
				while {
					let (playing, force) = *dialog.playing.lock().unwrap();
					playing && force
				} || dialog.keys.iter().all(|key| { key.is_pressed() }) {
					play_file(dialog.get_next_path());
					thread::sleep(Duration::from_secs_f32(dialog.delay));
				}
			}
			dialog.playing.lock().unwrap().0 = false;
			notify_redraw();
		});
	}
}