use std::{cmp::min, sync::{LazyLock, Mutex}};

use ratatui::{layout::Rect, style::{Color, Style}, text::{Line, Text}, widgets::{Block, BorderType, Clear, Padding, Paragraph, Widget}, Frame};

use crate::{component::block::BlockSingleton, state::acquire};

use super::BlockRender;

pub struct PlayingBlock { }

impl BlockSingleton for PlayingBlock {
	fn instance() -> std::sync::MutexGuard<'static, Self> {
		static BLOCK: LazyLock<Mutex<PlayingBlock>> = LazyLock::new(|| { Mutex::new(PlayingBlock {}) });
		BLOCK.lock().unwrap()
	}
}

impl BlockRender for PlayingBlock {
	fn render(&self, f: &mut Frame) {
		let app = acquire();
		let playing = &app.playing_file;
		let waves = &app.playing_wave;

		let mut lines: Vec<Line> = vec![];

		if playing.len() > 0 {
			lines.extend(playing.values().map(|(_id, file)| {
				Line::from(file.as_str()).style(Style::default().fg(Color::LightGreen))
			}));
		}

		if waves.len() > 0 {
			lines.extend(waves.values().map(|wave| {
				Line::from(wave.clone()).style(Style::default().fg(Color::LightBlue))
			}));
		}

		if lines.len() == 0 {
			return;
		}

		let len = lines.len();
		let area = f.area();
		let inner_height = min(5, len as u16);
		let block_area = Rect {
			x: 1,
			y: area.height - (4 + inner_height),
			width: area.width - 2,
			height: 2 + inner_height
		};
		Clear.render(block_area, f.buffer_mut());
		let paragraph = Paragraph::new(Text::from(lines)).block(Block::bordered().border_type(BorderType::Rounded).title(format!("Playing ({len})")).padding(Padding::horizontal(1)));
		f.render_widget(paragraph, block_area);
	}
}