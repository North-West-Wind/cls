use std::cmp::{max, min};

use crossterm::event::{KeyCode, KeyEvent};
use tui_input::Input;

use crate::state::{get_mut_app, AwaitInput, InputMode, Popup};

pub fn handle_tabs(event: KeyEvent) -> bool {
	match event.code {
		KeyCode::Char('a') => handle_add(),
		KeyCode::Char('d') => handle_remove(),
		KeyCode::Right => handle_move(true),
		KeyCode::Left => handle_move(false),
		_ => false
	}
}

fn handle_add() -> bool {
	let app = get_mut_app();
	app.input_mode = InputMode::EDITING;
	app.await_input = AwaitInput::ADD_TAB;
	app.input = Option::Some(Input::new(std::env::current_dir().unwrap().to_str().unwrap().to_string()));
	true
}

fn handle_remove() -> bool {
	let app = get_mut_app();
	let tab_selected = app.tab_selected;
	if tab_selected < app.config.tabs.len() {
		app.popup = Popup::DELETE_TAB;
		return true;
	}
	false
}

fn handle_move(right: bool) -> bool {
	let delta = if right { 1 } else { -1 };
	let app = get_mut_app();
	let tab_selected = app.tab_selected as i32;
	let new_selected = min(app.config.tabs.len() as i32 - 1, max(0, tab_selected + delta));
	if tab_selected != new_selected {
		app.tab_selected = new_selected as usize;
		return true;
	}
	false
}

pub fn delete_tab() {
	let app = get_mut_app();
	let tab_selected = app.tab_selected;
	app.files.as_mut().unwrap().remove(&app.config.tabs[tab_selected]);
	app.config.tabs.remove(tab_selected);
	if app.tab_selected >= app.config.tabs.len() {
		app.tab_selected = app.config.tabs.len() - 1;
	}
}