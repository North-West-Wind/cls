use std::cmp::min;

use ratatui::{layout::Rect, style::{Color, Style}, text::{Line, Text}, widgets::{Block, BorderType, Clear, Padding, Paragraph, Widget}, Frame};

use crate::state::get_app;

use super::BlockRender;

pub struct PlayingBlock {
	title: String,
}

impl Default for PlayingBlock {
	fn default() -> Self {
		Self {
			title: "Playing".to_string()
		}
	}
}

impl BlockRender for PlayingBlock {
	fn render(&self, f: &mut Frame) {
		let app = get_app();
		let playing = &app.playing_file;
		let waves = &app.playing_wave;

		let mut lines: Vec<Line> = vec![];

		if playing.len() > 0 {
			lines.extend(playing.values().map(|(_id, file)| {
				Line::from(file.as_str()).style(Style::default().fg(Color::LightGreen))
			}));
		}

		if waves.len() > 0 {
			lines.extend(waves.values().map(|(id, label)| {
				Line::from(format!("{} : {}", label, id)).style(Style::default().fg(Color::LightBlue))
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
		let paragraph = Paragraph::new(Text::from(lines)).block(Block::bordered().border_type(BorderType::Rounded).title(format!("{} ({len})", self.title)).padding(Padding::horizontal(1)));
		f.render_widget(paragraph, block_area);
	}
}