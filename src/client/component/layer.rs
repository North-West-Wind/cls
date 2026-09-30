use crossterm::event::{KeyCode, KeyEvent};

use crate::{client::{AtomicClientState, AtomicStaticBlocks, MainOpened, SelectionLayer, component::{block::{self, BlockNavigation, dialogs::DialogBlock, files::FilesBlock, results::ResultsBlock, search::SearchBlock, settings::SettingsBlock, tabs::TabsBlock, waves::WavesBlock}, popup::{confirm::ConfirmPopup, save::SavePopup}}}};

use super::{popup::{help::HelpPopup, PopupComponent}};

pub fn handle_key(client_state: AtomicClientState, blocks: AtomicStaticBlocks, event: KeyEvent) -> bool {
	match event.code {
		KeyCode::Up => key_navigate(client_state, blocks, 0, -1, event),
		KeyCode::Down => key_navigate(client_state, blocks, 0, 1, event),
		KeyCode::Left => key_navigate(client_state, blocks, -1, 0, event),
		KeyCode::Right => key_navigate(client_state, blocks, 1, 0, event),
		KeyCode::Enter => navigate_layer(client_state, false),
		KeyCode::Char('q')|KeyCode::Esc => navigate_layer(client_state, true),
		KeyCode::Char('?') => {
			client_state.write().popup_manager.push(PopupComponent::Help(HelpPopup::default()));
			return true;
		},
		KeyCode::Char('s') => {
			save(client_state);
			return true;
		},
		KeyCode::Char('c') => {
			let mut client_state = client_state.write();
			client_state.settings_opened = !client_state.settings_opened;
			if !client_state.settings_opened && client_state.selected_block == SettingsBlock::ID {
				client_state.selected_block = client_state.main_opened.id(SettingsBlock::ID);
			}
			return true;
		},
		KeyCode::Char('w') => {
			toggle_main_opened(client_state, MainOpened::Wave);
			return true;
		},
		KeyCode::Char('t') => {
			toggle_main_opened(client_state, MainOpened::Dialog);
			return true;
		},
		KeyCode::Char('\'') => {
			toggle_main_opened(client_state, MainOpened::Search);
			return true;
		},
		KeyCode::Char('\\') => {
			let mut client_state = client_state.write();
			if client_state.main_opened == MainOpened::Log {
				client_state.main_opened = MainOpened::File;
			} else {
				client_state.main_opened = MainOpened::Log;
			}
			return true;
		},
		_ => block::handle_key(client_state, blocks, event)
	}
}

fn key_navigate(client_state: AtomicClientState, blocks: AtomicStaticBlocks, dx: i16, dy: i16, event: KeyEvent) -> bool {
	if dx == 0 && dy == 0 { return false }
	let mut result = navigate_block(client_state.clone(), blocks.clone(), dx, dy);
	if !result {
		result = block::handle_key(client_state, blocks, event);
	}
	return result;
}

fn navigate_block(client_state: AtomicClientState, blocks: AtomicStaticBlocks, dx: i16, dy: i16) -> bool {
	if dx == 0 && dy == 0 { return false }
	let old_block = { client_state.read().selected_block };
	let new_block = block::navigate_block(client_state.clone(), blocks, old_block, dx, dy);

	if old_block != new_block {
		client_state.write().selected_block = new_block;
		return true;
	}
	false
}

pub fn navigate_layer(atomic_client_state: AtomicClientState, escape: bool) -> bool {
	let mut client_state = atomic_client_state.write();
	if escape {
		match client_state.selection_layer {
			SelectionLayer::Block => {
				let atomic_clone = atomic_client_state.clone();
				client_state.popup_manager.push(PopupComponent::Confirm(ConfirmPopup::new("Quit?", "quit", move || atomic_clone.write().exit())));
				true
			},
			SelectionLayer::Content => {
				client_state.selection_layer = SelectionLayer::Block;
				true
			}
		}
	} else {
		match client_state.selection_layer {
			SelectionLayer::Block => {
				client_state.selection_layer = SelectionLayer::Content;
				true
			},
			SelectionLayer::Content => false,
		}
	}
}

fn save(client_state: AtomicClientState) {
	let mut client_state = client_state.write();
	client_state.save_config();
	client_state.popup_manager.push(PopupComponent::Save(SavePopup::new(true)));
}

fn toggle_main_opened(client_state: AtomicClientState, main_opened: MainOpened) {
	let mut client_state = client_state.write();
	if client_state.main_opened == main_opened {
		client_state.main_opened = MainOpened::File;
	} else {
		client_state.main_opened = main_opened;
	}
	if client_state.selected_block == FilesBlock::ID || client_state.selected_block == WavesBlock::ID || client_state.selected_block == DialogBlock::ID || client_state.selected_block == ResultsBlock::ID {
		client_state.selected_block = client_state.main_opened.id(client_state.selected_block);
	}

	if client_state.main_opened == MainOpened::Search {
		client_state.selected_block = SearchBlock::ID;
	} else if client_state.selected_block == SearchBlock::ID {
		client_state.selected_block = TabsBlock::ID;
	}
}