use std::cmp::{max, min};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::{handler::pulseaudio::set_volume_percentage, state::get_mut_app};

pub fn handle_volume(event: KeyEvent) -> bool {
	match event.code {
		KeyCode::Right => change_volume(if event.modifiers.contains(KeyModifiers::CONTROL) { 5 } else { 1 }),
		KeyCode::Left => change_volume(if event.modifiers.contains(KeyModifiers::CONTROL) { -5 } else { -1 }),
		_ => false
	}
}

fn change_volume(delta: i16) -> bool {
	let app = get_mut_app();
	let old_volume = app.config.volume as i16;
	let new_volume = min(200, max(0, old_volume + delta));
	if new_volume != old_volume {
		set_volume_percentage(new_volume as u32);
		app.config.volume = new_volume as u32;
		return true
	}
	false
}