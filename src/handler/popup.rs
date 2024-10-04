use crossterm::event::{KeyCode, KeyEvent};

use crate::state::{get_app, get_mut_app, Popup};

use super::{block::tabs::delete_tab, navigate::navigate_layer};

pub fn handle_popup_key_event(event: KeyEvent) -> bool {
	let app = get_app();
	match app.popup {
		Popup::HELP => handle_help(event),
		Popup::QUIT => handle_quit(event),
		Popup::DELETE_TAB => handle_delete_tab(event),
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
		_ => navigate_layer(true)
	}
}

fn handle_help(event: KeyEvent) -> bool {
	let app = get_mut_app();
	match event.code {
		KeyCode::Char('q')|KeyCode::Esc => {
			app.popup = Popup::NONE;
			return true
		},
		_ => false
	}
}

fn handle_delete_tab(event: KeyEvent) -> bool {
	let app = get_mut_app();
	match event.code {
		KeyCode::Char('y') => {
			app.popup = Popup::NONE;
			delete_tab();
			return true
		},
		_ => navigate_layer(true)
	}
}