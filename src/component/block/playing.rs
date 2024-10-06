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
		let playing = app.playing.as_ref().unwrap();
		if playing.len() == 0 {
			return;
		}
		let len = playing.len();
		let area = f.area();
		let inner_height = min(5, len as u16);
		let block_area = Rect {
			x: 1,
			y: area.height - (4 + inner_height),
			width: area.width - 2,
			height: 2 + inner_height
		};
		Clear.render(block_area, f.buffer_mut());
		let mut lines = vec![];
		let playing_list = playing.values().map(|(path, _)| { path }).collect::<Vec<&String>>();
		for ii in 0..inner_height {
			lines.push(Line::from(playing_list.get(ii as usize).unwrap().as_str()).style(Style::default().fg(Color::LightGreen)));
		}
		let paragraph = Paragraph::new(Text::from(lines)).block(Block::bordered().border_type(BorderType::Rounded).title(format!("{} ({len})", self.title)).padding(Padding::horizontal(1)));
		f.render_widget(paragraph, block_area);
	}
}