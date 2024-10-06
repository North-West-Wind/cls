use crossterm::event::{KeyCode, KeyEvent};
use files::FilesBlock;
use help::HelpBlock;
use playing::PlayingBlock;
use ratatui::{layout::Rect, style::{Color, Style}, widgets::BorderType, Frame};
use tabs::TabsBlock;
use volume::VolumeBlock;

use crate::state::{get_app, SelectionLayer};

use super::{layer, popup::{help::HelpPopup, set_popup, PopupComponent}};

pub mod files;
pub mod help;
pub mod playing;
pub mod tabs;
pub mod volume;

pub enum BlockComponent {
	Volume(VolumeBlock),
	Tabs(TabsBlock),
	Files(FilesBlock),
	Help(HelpBlock),
	Playing(PlayingBlock),
}

pub trait BlockRender {
	fn render(&self, f: &mut Frame);
}

pub trait BlockRenderArea {
	fn render_area(&self, f: &mut Frame, area: Rect);
}

pub trait BlockHandleKey {
	fn handle_key(&self, event: KeyEvent) -> bool;
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
	fn render_area(&self, f: &mut Frame, area: Rect) {
		match self {
			BlockComponent::Volume(block) => block.render_area(f, area),
			BlockComponent::Tabs(block) => block.render_area(f, area),
			BlockComponent::Files(block) => block.render_area(f, area),
			BlockComponent::Help(block) => block.render_area(f, area),
			_ => (),
		}
	}
}

impl BlockHandleKey for BlockComponent {
	fn handle_key(&self, event: KeyEvent) -> bool {
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