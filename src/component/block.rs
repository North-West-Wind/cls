use std::sync::MutexGuard;

use crossterm::event::{KeyCode, KeyEvent};
use files::FilesBlock;
use ratatui::{layout::Rect, Frame};
use settings::SettingsBlock;
use tabs::TabsBlock;
use info::InfoBlock;

use crate::{component::block::waves::WavesBlock};

use super::{layer, popup::{help::HelpPopup, set_popup, PopupComponent}};

pub mod files;
pub mod help;
pub mod playing;
pub mod settings;
pub mod tabs;
pub mod info;
pub mod waves;

pub trait BlockSingleton {
	fn instance() -> MutexGuard<'static, Self>;
}

pub trait BlockRender {
	fn render(&self, f: &mut Frame);
}

pub trait BlockRenderArea {
	fn render_area(&mut self, f: &mut Frame, area: Rect);
}

pub trait BlockHandleKey {
	fn handle_key(&mut self, event: KeyEvent) -> bool;
}

pub trait BlockNavigation {
	const ID: u8;
	fn navigate_block(&self, dx: i16, dy: i16) -> u8;
}

pub fn handle_key(block_id: u8, event: KeyEvent) -> bool {
	use KeyCode::*;
	match event.code {
		Char('q')|KeyCode::Esc => layer::navigate_layer(true),
		Char('?') => {
			set_popup(PopupComponent::Help(HelpPopup::default()));
			return true;
		},
		_ => match block_id {
			InfoBlock::ID => InfoBlock::instance().handle_key(event),
			TabsBlock::ID => TabsBlock::instance().handle_key(event),
			FilesBlock::ID => FilesBlock::instance().handle_key(event),
			SettingsBlock::ID => SettingsBlock::instance().handle_key(event),
			WavesBlock::ID => WavesBlock::instance().handle_key(event),
			_ => false,
		}
	}
}

pub fn navigate_block(block_id: u8, dx: i16, dy: i16) -> u8 {
	match block_id {
		InfoBlock::ID => InfoBlock::instance().navigate_block(dx, dy),
		TabsBlock::ID => TabsBlock::instance().navigate_block(dx, dy),
		FilesBlock::ID => FilesBlock::instance().navigate_block(dx, dy),
		SettingsBlock::ID => SettingsBlock::instance().navigate_block(dx, dy),
		WavesBlock::ID => WavesBlock::instance().navigate_block(dx, dy),
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