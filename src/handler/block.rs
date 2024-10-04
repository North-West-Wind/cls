use std::path::Path;

use crossterm::event::{KeyCode, KeyEvent};
use files::handle_files;
use normpath::PathExt;
use tabs::handle_tabs;

use crate::{state::{get_app, get_mut_app, AwaitInput, CondvarPair, Popup}, util::scan_tab};

use super::navigate::{navigate_layer, navigate_popup};

pub mod files;
pub mod tabs;

pub fn handle_block_key_event(pair: CondvarPair, event: KeyEvent) -> bool {
	match event.code {
		KeyCode::Enter => navigate_layer(false),
		KeyCode::Char('q')|KeyCode::Esc => navigate_layer(true),
		KeyCode::Char('?') => navigate_popup(Popup::HELP),
		_ => match get_app().block_selected {
			1 => handle_tabs(event), // tabs
			2 => handle_files(pair, event), // files
			_ => false
		}
	}
}

pub fn handle_input_return() {
	let app = get_app();
	match app.await_input {
		AwaitInput::ADD_TAB => handle_input_return_add_tab(app.input.as_ref().unwrap().value().to_string()),
		_ => (),
	}
}

fn handle_input_return_add_tab(str: String) {
	let app = get_mut_app();
	app.config.tabs.push(Path::new(&str).normalize().unwrap().into_os_string().into_string().unwrap());
	app.tab_selected = app.config.tabs.len() - 1;
	let _ = scan_tab(app.tab_selected);
}