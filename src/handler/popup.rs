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
		_ => {
			exit_popup();
			return true
		}
	}
}

fn handle_help(event: KeyEvent) -> bool {
	let app = get_mut_app();
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