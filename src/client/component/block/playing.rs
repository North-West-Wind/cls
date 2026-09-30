use std::{cmp::min, format};

use ratatui::{layout::Rect, style::Style, text::{Line, Text}, widgets::{Block, BorderType, Clear, Padding, Paragraph, Widget}, Frame};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

use crate::client::ClientState;

use super::BlockRender;

#[derive(Default)]
pub struct PlayingBlock { }

impl BlockRender for PlayingBlock {
	fn render(&self, client_state: &ClientState, f: &mut Frame) {
		let lines = client_state.playing.par_iter().map(|(_, (text, color))| {
			Line::from(text.clone()).style(Style::default().fg(*color))
		}).collect::<Vec<_>>();

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