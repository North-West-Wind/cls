use crossterm::event::{KeyCode, KeyEvent};
use files::FilesBlock;
use ratatui::{Frame, layout::Rect};
use settings::SettingsBlock;
use tabs::TabsBlock;
use info::InfoBlock;

use crate::client::{AtomicClientState, AtomicStaticBlocks, ClientState, component::block::{dialogs::DialogBlock, results::ResultsBlock, search::SearchBlock, waves::WavesBlock}};

use super::{layer, popup::{help::HelpPopup, PopupComponent}};

pub mod dialogs;
pub mod files;
pub mod help;
pub mod playing;
pub mod results;
pub mod search;
pub mod settings;
pub mod tabs;
pub mod info;
pub mod log;
pub mod waves;

pub trait BlockRender {
	fn render(&self, client_state: &ClientState, f: &mut Frame);
}

pub trait BlockRenderArea {
	fn render_area(&mut self, client_state: &ClientState, f: &mut Frame, area: Rect);
}

pub trait BlockHandleKey {
	fn handle_key(&mut self, client_state: AtomicClientState, event: KeyEvent) -> bool;
}

pub trait BlockNavigation {
	const ID: u8;
	fn navigate_block(&self, client_state: AtomicClientState, dx: i16, dy: i16) -> u8;
}

pub fn handle_key(client_state: AtomicClientState, blocks: AtomicStaticBlocks, event: KeyEvent) -> bool {
	use KeyCode::*;
	match event.code {
		Char('q')|KeyCode::Esc => layer::navigate_layer(client_state, true),
		Char('?') => {
			client_state.write().popup_manager.push(PopupComponent::Help(HelpPopup::default()));
			return true;
		},
		_ => {
			let mut blocks = blocks.write();
			match { client_state.read().selected_block } {
				InfoBlock::ID => blocks.info.handle_key(client_state, event),
				TabsBlock::ID => blocks.tabs.handle_key(client_state, event),
				FilesBlock::ID => blocks.files.handle_key(client_state, event),
				SettingsBlock::ID => blocks.settings.handle_key(client_state, event),
				WavesBlock::ID => blocks.waves.handle_key(client_state, event),
				DialogBlock::ID => blocks.dialogs.handle_key(client_state, event),
				SearchBlock::ID => blocks.search.handle_key(client_state, event),
				ResultsBlock::ID => blocks.results.handle_key(client_state, event),
				_ => false,
			}
		}
	}
}

pub fn navigate_block(client_state: AtomicClientState, blocks: AtomicStaticBlocks, block_id: u8, dx: i16, dy: i16) -> u8 {
	let blocks = blocks.read();
	match block_id {
		InfoBlock::ID => blocks.info.navigate_block(client_state, dx, dy),
		TabsBlock::ID => blocks.tabs.navigate_block(client_state, dx, dy),
		FilesBlock::ID => blocks.files.navigate_block(client_state, dx, dy),
		SettingsBlock::ID => blocks.settings.navigate_block(client_state, dx, dy),
		WavesBlock::ID => blocks.waves.navigate_block(client_state, dx, dy),
		DialogBlock::ID => blocks.dialogs.navigate_block(client_state, dx, dy),
		SearchBlock::ID => blocks.search.navigate_block(client_state, dx, dy),
		ResultsBlock::ID => blocks.results.navigate_block(client_state, dx, dy),
		_ => block_id
	}
}

pub(self) fn loop_index(index: usize, delta: i32, max: usize) -> usize {
	let mut new_index = index as i32 + delta;
	if new_index < 0 {
		let factor = new_index / max as i32;
		new_index += max as i32 * (factor + 1);
	}
	new_index %= max as i32;
	new_index as usize
}