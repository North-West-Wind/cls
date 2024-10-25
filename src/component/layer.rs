use std::cmp::{max, min};

use crossterm::event::{KeyCode, KeyEvent};

use crate::{state::{get_app, get_mut_app, SelectionLayer}, util::threads::spawn_save_thread};

use super::popup::{help::HelpPopup, quit::QuitPopup, set_popup, PopupComponent};

pub fn handle_key(event: KeyEvent) -> bool {
	match event.code {
		KeyCode::Up => key_navigate(0, -1),
		KeyCode::Down => key_navigate(0, 1),
		KeyCode::Left => key_navigate(-1, 0),
		KeyCode::Right => key_navigate(1, 0),
		KeyCode::Enter => navigate_layer(false),
		KeyCode::Char('q')|KeyCode::Esc => navigate_layer(true),
		KeyCode::Char('?') => {
			set_popup(PopupComponent::Help(HelpPopup::default()));
			return true;
		},
		KeyCode::Char('s') => {
			spawn_save_thread();
			return true;
		},
		KeyCode::Char('c') => {
			let app = get_mut_app();
			app.settings_opened = !app.settings_opened;
			return true;
		},
		_ => false
	}
}

fn key_navigate(dx: i16, dy: i16) -> bool {
	if dx == 0 && dy == 0 { return false }
	let app = get_app();
	if app.selection_layer == SelectionLayer::Block {
		navigate_block(dx, dy)
	} else {
		false
	}
}

fn navigate_block(dx: i16, dy: i16) -> bool {
	if dx == 0 && dy == 0 { return false }
	let app = get_mut_app();
	let old_block = app.block_selected;
	let new_block: i16;
	if dy > 0 {
		// moving down
		new_block = min(2, old_block as i16 + dy);
	} else if dy < 0 {
		// moving up
		new_block = max(0, old_block as i16 + dy * (if old_block == 3 { 2 } else { 1 }));
	} else if dx > 0 && old_block == 2 || dx < 0 && old_block == 3 {
		new_block = old_block as i16 + dx;
	} else {
		new_block = old_block as i16;
	}

	if new_block as u8 != old_block {
		app.block_selected = new_block as u8;
		return true
	}
	false
}

pub fn navigate_layer(escape: bool) -> bool {
	let app = get_mut_app();
	if escape {
		match app.selection_layer {
			SelectionLayer::Block => {
				set_popup(PopupComponent::Quit(QuitPopup::default()));
				return true
			},
			SelectionLayer::Content => {
				app.selection_layer = SelectionLayer::Block;
				return true
			}
		}
	} else {
		match app.selection_layer {
			SelectionLayer::Block => {
				app.selection_layer = SelectionLayer::Content;
				return true
			},
			SelectionLayer::Content => return false,
		}
	}
}