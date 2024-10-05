use mki::Keyboard;
use pulseaudio::play_file;

use crate::state::{get_mut_app, CondvarPair};

pub mod block;
pub mod input;
pub mod layer;
pub mod navigate;
pub mod popup;
pub mod pulseaudio;

pub fn handle_inputbot(pair: CondvarPair, key: Keyboard) {
	let app = get_mut_app();
	if app.recording {
		use Keyboard::*;
		match key {
			Enter|Escape => false,
			Other(_c) => false,
			_ => app.recorded.as_mut().unwrap().insert(key)
		};

		let (lock, cvar) = &*pair;
		let mut shared = lock.lock().unwrap();
		(*shared).redraw = true;
		cvar.notify_all();
	} else {
		for (path, keys) in app.hotkey.as_ref().unwrap() {
			if keys.iter().all(|key| { key.is_pressed() }) {
				play_file(path);
			}
		}
	}
}