use ratatui::{layout::Rect, text::Line, widgets::{Block, BorderType, Clear, Padding, Paragraph, Widget}, Frame};
use std::cmp::max;

use crate::component::popup::PopupRender;

pub struct WavePopup {
	index: u32,
}

impl WavePopup {
	fn new(index: u32) -> Self {
		Self {
			index
		}
	}
}

impl PopupRender for WavePopup {
	fn render(&self, f: &mut Frame) {
		let mut lines = vec![
			Line::from("up / down - select"),
			Line::from("left / right - change type"),
			Line::from("enter - change frequency")
		];

		let area = f.area();
		let width = lines.iter()
			.map(|line| { line.width() })
			.fold(0, |acc, width| max(acc, width)) as u16;
		let height = lines.len() as u16;

		let popup_area = Rect {
			x: (area.width - width) / 2,
			y: (area.height - height) / 2,
			width,
			height
		};

		Clear.render(popup_area, f.buffer_mut());
		f.render_widget(Paragraph::new(lines).block(Block::bordered().padding(Padding::uniform(1)).border_type(BorderType::Rounded)), popup_area);
	}
}