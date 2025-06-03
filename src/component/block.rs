use crossterm::event::{KeyCode, KeyEvent};
use files::FilesBlock;
use help::HelpBlock;
use playing::PlayingBlock;
use ratatui::{layout::Rect, style::{Color, Style}, widgets::BorderType, Frame};
use settings::SettingsBlock;
use tabs::TabsBlock;
use volume::VolumeBlock;

use crate::{component::block::waves::WavesBlock, state::{get_app, SelectionLayer}};

use super::{layer, popup::{help::HelpPopup, set_popup, PopupComponent}};

pub mod files;
pub mod help;
pub mod playing;
pub mod settings;
pub mod tabs;
pub mod volume;
pub mod waves;

pub enum BlockComponent {
	Volume(VolumeBlock),
	Tabs(TabsBlock),
	Files(FilesBlock),
	Settings(SettingsBlock),
	Help(HelpBlock),
	Playing(PlayingBlock),
	Waves(WavesBlock),
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

impl BlockRender for BlockComponent {
	fn render(&self, f: &mut Frame) {
		match self {
			BlockComponent::Playing(block) => block.render(f),
			_ => (),
		}
	}
}

impl BlockRenderArea for BlockComponent {
	fn render_area(&mut self, f: &mut Frame, area: Rect) {
		match self {
			BlockComponent::Volume(block) => block.render_area(f, area),
			BlockComponent::Tabs(block) => block.render_area(f, area),
			BlockComponent::Files(block) => block.render_area(f, area),
			BlockComponent::Settings(block) => block.render_area(f, area),
			BlockComponent::Help(block) => block.render_area(f, area),
			BlockComponent::Waves(block) => block.render_area(f, area),
			_ => (),
		}
	}
}

impl BlockHandleKey for BlockComponent {
	fn handle_key(&mut self, event: KeyEvent) -> bool {
		use BlockComponent::*;
		use KeyCode::*;
		match event.code {
			Char('q')|KeyCode::Esc => layer::navigate_layer(true),
			Char('?') => {
				set_popup(PopupComponent::Help(HelpPopup::default()));
				return true;
			},
			_ => match self {
				Volume(block) => block.handle_key(event),
				Tabs(block) => block.handle_key(event),
				Files(block) => block.handle_key(event),
				Settings(block) => block.handle_key(event),
				Waves(block) => block.handle_key(event),
				_ => false,
			}
		}
	}
}

impl BlockNavigation for BlockComponent {
	const ID: u8 = 0; // Unused

	fn navigate_block(&self, dx: i16, dy: i16) -> u8 {
		use BlockComponent::*;
		match self {
			Volume(block) => block.navigate_block(dx, dy),
			Tabs(block) => block.navigate_block(dx, dy),
			Files(block) => block.navigate_block(dx, dy),
			Settings(block) => block.navigate_block(dx, dy),
			Waves(block) => block.navigate_block(dx, dy),
			_ => Self::ID
		}
	}
}

pub(self) fn borders(id: u8) -> (BorderType, Style) {
	let app = get_app();
	let style = Style::default().fg(
		if app.block_selected == id {
			Color::White
		} else {
			Color::DarkGray
		}
	);
	let border_type = if app.block_selected == id {
		if app.selection_layer == SelectionLayer::Content {
			BorderType::Double
		} else {
			BorderType::Thick
		}
	} else {
		BorderType::Rounded
	};
	(border_type, style)
}

impl BlockComponent {
	pub fn _volume_selected(&self) -> Option<usize> {
		match self {
			BlockComponent::Volume(block) => Option::Some(block.selected),
			_ => Option::None
		}
	}

	pub fn file_selected(&self) -> Option<usize> {
		match self {
			BlockComponent::Files(block) => Option::Some(block.selected),
			_ => Option::None
		}
	}

	pub fn set_file_selected(&mut self, selected: usize) {
		match self {
			BlockComponent::Files(block) => block.selected = selected,
			_ => ()
		}
	}

	pub fn tab_selected(&self) -> Option<usize> {
		match self {
			BlockComponent::Tabs(block) => Option::Some(block.selected),
			_ => Option::None
		}
	}

	pub fn set_tab_selected(&mut self, selected: usize) {
		match self {
			BlockComponent::Tabs(block) => block.selected = selected,
			_ => ()
		}
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