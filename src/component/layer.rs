use crossterm::event::{KeyCode, KeyEvent};

use crate::{component::{block::{self, files::FilesBlock, settings::SettingsBlock, waves::WavesBlock, BlockNavigation}, popup::confirm::{ConfirmAction, ConfirmPopup}}, state::{acquire, SelectionLayer}, util::threads::spawn_save_thread};

use super::{popup::{help::HelpPopup, set_popup, PopupComponent}};

pub fn handle_key(event: KeyEvent) -> bool {
	match event.code {
		KeyCode::Up => key_navigate(0, -1, event),
		KeyCode::Down => key_navigate(0, 1, event),
		KeyCode::Left => key_navigate(-1, 0, event),
		KeyCode::Right => key_navigate(1, 0, event),
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
			let mut app = acquire();
			app.settings_opened = !app.settings_opened;
			if !app.settings_opened && app.block_selected == SettingsBlock::ID {
				app.block_selected = if app.waves_opened { WavesBlock::ID } else { FilesBlock::ID };
			}
			return true;
		},
		KeyCode::Char('w') => {
			let mut app = acquire();
			app.waves_opened = !app.waves_opened;
			if app.waves_opened && app.block_selected == FilesBlock::ID {
				app.block_selected = WavesBlock::ID;
			} else if !app.waves_opened && app.block_selected == WavesBlock::ID {
				app.block_selected = FilesBlock::ID;
			}
			return true;
		},
		KeyCode::Char('\\') => {
			let mut app = acquire();
			app.logs_opened = !app.logs_opened;
			return true;
		},
		_ => {
			let selected = { acquire().block_selected };
			block::handle_key(selected, event)
		}
	}
}

fn key_navigate(dx: i16, dy: i16, event: KeyEvent) -> bool {
	if dx == 0 && dy == 0 { return false }
	let mut result = navigate_block(dx, dy);
	if !result {
		let selected = { acquire().block_selected };
		result = block::handle_key(selected, event);
	}
	return result;
}

fn navigate_block(dx: i16, dy: i16) -> bool {
	if dx == 0 && dy == 0 { return false }
	let old_block = { acquire().block_selected };
	let new_block = block::navigate_block(old_block, dx, dy);

	if old_block != new_block {
		acquire().block_selected = new_block;
		return true;
	}
	false
}

pub fn navigate_layer(escape: bool) -> bool {
	let mut app = acquire();
	if escape {
		match app.selection_layer {
			SelectionLayer::Block => {
				set_popup(PopupComponent::Confirm(ConfirmPopup::new(ConfirmAction::Quit)));
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