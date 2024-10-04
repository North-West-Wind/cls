use crossterm::event::{KeyCode, KeyEvent};

use crate::state::Popup;

use super::navigate::{key_navigate, navigate_layer, navigate_popup};

pub fn handle_layer_key_event(event: KeyEvent) -> bool {
	match event.code {
		KeyCode::Up => key_navigate(0, -1),
		KeyCode::Down => key_navigate(0, 1),
		KeyCode::Enter => navigate_layer(false),
		KeyCode::Char('q')|KeyCode::Esc => navigate_layer(true),
		KeyCode::Char('?') => navigate_popup(Popup::HELP),
		_ => false
	}
}