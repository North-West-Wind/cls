use std::sync::{Arc, Mutex};

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{style::Color, widgets::{Block, Borders, Padding}};

use crate::{component::block::{borders, settings::SettingsBlock, tabs::TabsBlock, BlockHandleKey, BlockNavigation, BlockRenderArea}, state::get_app, util::waveform::{play_wave, Wave, WaveType, Waveform}};

pub struct WavesBlock {
	test_wave: Waveform
}

impl Default for WavesBlock {
	fn default() -> Self {
		Self {
			test_wave: Waveform {
				label: "test".to_string(),
				keys: vec![],
				waves: vec![Wave {
					wave_type: WaveType::Sine,
					frequency: 1000.0
				}],
				volume: 80,
				playing: Arc::new(Mutex::new(false))
			}
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

impl BlockHandleKey for WavesBlock {
	fn handle_key(&mut self, event: KeyEvent) -> bool {
		match event.code {
			KeyCode::Enter => {
				play_wave(self.test_wave.clone(), true);
				true
			},
			_ => false
		}
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