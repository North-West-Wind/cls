use ratatui::{style::Color, widgets::{Block, Borders, Padding}};

use crate::{component::block::{borders, settings::SettingsBlock, tabs::TabsBlock, BlockNavigation, BlockRenderArea}, state::get_app};

pub struct WavesBlock {
}

impl Default for WavesBlock {
	fn default() -> Self {
		Self {

		}
	}
}

impl BlockRenderArea for WavesBlock {
	fn render_area(&mut self, f: &mut ratatui::Frame, area: ratatui::prelude::Rect) {
		let app = get_app();
		let selected = app.block_selected == Self::ID;

		let (border_type, border_style) = borders(Self::ID);
		let block = Block::default()
			.title("Waveforms")
			.borders(Borders::ALL)
			.border_type(border_type)
			.border_style(border_style.fg(if selected { Color::LightBlue } else { Color::Blue }))
			.padding(Padding::new(2, 2, 1, 1));

		f.render_widget(block, area);
	}
}

impl BlockNavigation for WavesBlock {
	const ID: u8 = 6;

	fn navigate_block(&self, dx: i16, dy: i16) -> u8 {
		if dy < 0 {
			return TabsBlock::ID;
		}
		if dx > 0 && get_app().settings_opened {
			return SettingsBlock::ID;
		}
		Self::ID
	}
}