use std::{cmp::{max, min}, collections::HashMap};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::{handler::pulseaudio::set_volume_percentage, state::get_mut_app, util::selected_file_path};

pub fn handle_volume(event: KeyEvent) -> bool {
	match event.code {
		KeyCode::Right => change_volume(if event.modifiers.contains(KeyModifiers::CONTROL) { 5 } else { 1 }),
		KeyCode::Left => change_volume(if event.modifiers.contains(KeyModifiers::CONTROL) { -5 } else { -1 }),
		KeyCode::Up => select_volume(0),
		KeyCode::Down => select_volume(1),
		_ => false
	}
}

fn select_volume(selection: usize) -> bool {
	let app = get_mut_app();
	if app.volume_selected != selection {
		if selection == 1 {
			let selected_file = selected_file_path();
			if selected_file.is_empty() {
				return false;
			}
		}
		app.volume_selected = selection;
		return true;
	}
	false
}

fn change_volume(delta: i16) -> bool {
	let app = get_mut_app();
	if app.volume_selected == 1 {
		return change_file_volume(delta);
	}
	let old_volume = app.config.volume as i16;
	let new_volume = min(200, max(0, old_volume + delta));
	if new_volume != old_volume {
		set_volume_percentage(new_volume as u32);
		app.config.volume = new_volume as u32;
		return true
	}
	false
}

fn change_file_volume(delta: i16) -> bool {
	let selected_file = selected_file_path();
	if selected_file.is_empty() {
		return false;
	}
	let app = get_mut_app();
	if app.config.file_volume.is_none() {
		app.config.file_volume = Option::Some(HashMap::new());
	}
	let map = app.config.file_volume.as_mut().unwrap();
	let old_volume = map.get(&selected_file).unwrap_or(&100);
	let new_volume = min(100, max(0, (*old_volume) as i16 + delta)) as usize;
	if new_volume != *old_volume {
		map.insert(selected_file, new_volume);
		return true
	}
	false
}