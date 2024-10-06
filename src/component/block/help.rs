use ratatui::{layout::Rect, style::{Color, Style}, widgets::Paragraph, Frame};

use super::BlockRenderArea;

pub struct HelpBlock { }

impl Default for HelpBlock {
	fn default() -> Self {
		Self { }
	}
}

impl BlockRenderArea for HelpBlock {
	fn render_area(&self, f: &mut Frame, area: Rect) {
		let paragraph = Paragraph::new("? for help, q to quit")
			.style(Style::default().fg(Color::DarkGray));
		f.render_widget(paragraph, area);
	}
}