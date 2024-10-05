use std::collections::HashMap;

use crossterm::event::{KeyCode, KeyEvent};

use crate::{global_input::keyboard_to_string, state::{get_app, get_mut_app, Popup}, util::selected_file_path};

use super::{block::tabs::delete_tab, navigate::navigate_layer};

pub fn handle_popup_key_event(event: KeyEvent) -> bool {
	let app = get_app();
	match app.popup {
		Popup::HELP => handle_help(event),
		Popup::QUIT => handle_quit(event),
		Popup::DELETE_TAB => handle_delete_tab(event),
		Popup::KEY_BIND => handle_key_bind(event),
		_ => false
	}
}

fn handle_quit(event: KeyEvent) -> bool {
	let app = get_mut_app();
	match event.code {
		KeyCode::Char('y') => {
			app.running = false;
			return false
		},
		_ => {
			exit_popup();
			return true
		}
	}
}

fn handle_help(event: KeyEvent) -> bool {
	match event.code {
		KeyCode::Char('q')|KeyCode::Esc => {
			exit_popup();
			return true
		},
		_ => false
	}
}

fn handle_delete_tab(event: KeyEvent) -> bool {
	match event.code {
		KeyCode::Char('y') => {
			exit_popup();
			delete_tab();
			return true
		},
		_ => navigate_layer(true)
	}
}

fn exit_popup() {
	let app = get_mut_app();
	app.popup = Popup::NONE;
}

fn handle_key_bind(event: KeyEvent) -> bool {
	let app = get_mut_app();
	match event.code {
		KeyCode::Enter => {
			if !app.recording {
				app.recording = true;
			} else {
				app.recording = false;
				set_key_bind();
				exit_popup();
			}
			return true;
		},
		KeyCode::Esc => {
			if app.recording {
				app.recording = false;
			} else {
				app.recorded.as_mut().unwrap().clear();
				exit_popup();
			}
			return true;
		},
		KeyCode::Char('r') => {
			if app.recording {
				return false;
			}
			app.recorded.as_mut().unwrap().clear();
			return true;
		},
		_ => false
	}
}

fn set_key_bind() {
	let path = selected_file_path();
	if path.is_empty() {
		return;
	}
	let app = get_mut_app();
	if app.config.file_key.is_none() {
		app.config.file_key = Option::Some(HashMap::new());
	}
	let map = app.config.file_key.as_mut().unwrap();
	let recorded = app.recorded.as_ref().unwrap();
	map.insert(path.clone(), recorded.into_iter().map(|key| { keyboard_to_string(*key) }).collect::<Vec<String>>());
	let mut keyboard = vec![];
	for key in recorded {
		keyboard.push(*key);
	}
	app.hotkey.as_mut().unwrap().insert(path, keyboard);
}