use crossterm::event::{KeyCode, KeyEvent};
use files::FilesBlock;
use help::HelpBlock;
use playing::PlayingBlock;
use ratatui::{layout::Rect, style::{Color, Style}, widgets::BorderType, Frame};
use settings::SettingsBlock;
use tabs::TabsBlock;
use volume::VolumeBlock;

use crate::state::{get_app, SelectionLayer};

use super::{layer, popup::{help::HelpPopup, set_popup, PopupComponent}};

pub mod files;
pub mod help;
pub mod playing;
pub mod settings;
pub mod tabs;
pub mod volume;

pub enum BlockComponent {
	Volume(VolumeBlock),
	Tabs(TabsBlock),
	Files(FilesBlock),
	Settings(SettingsBlock),
	Help(HelpBlock),
	Playing(PlayingBlock),
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
			_ => (),
		}
	}
}

impl BlockHandleKey for BlockComponent {
	fn handle_key(&mut self, event: KeyEvent) -> bool {
		match event.code {
			KeyCode::Char('q')|KeyCode::Esc => layer::navigate_layer(true),
			KeyCode::Char('?') => {
				set_popup(PopupComponent::Help(HelpPopup::default()));
				return true;
			},
			_ => match self {
				BlockComponent::Volume(block) => block.handle_key(event),
				BlockComponent::Tabs(block) => block.handle_key(event),
				BlockComponent::Files(block) => block.handle_key(event),
				BlockComponent::Settings(block) => block.handle_key(event),
				_ => false,
			}
		}
	}
}

pub(self) fn border_style(id: u8) -> Style {
	let app = get_app();
	Style::default().fg(
		if app.block_selected == id {
			Color::White
		} else {
			Color::DarkGray
		}
	)
}

pub(self) fn border_type(id: u8) -> BorderType {
	let app = get_app();
	if app.block_selected == id && app.selection_layer == SelectionLayer::Content {
		BorderType::Double
	} else {
		BorderType::Rounded
	}
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