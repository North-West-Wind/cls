use ratatui::{layout::Rect, style::{Color, Style}, widgets::Paragraph, Frame};

use crate::state::get_mut_app;

use super::BlockRenderArea;

pub struct HelpBlock { }

impl Default for HelpBlock {
	fn default() -> Self {
		Self { }
	}
}

impl BlockRenderArea for HelpBlock {
	fn render_area(&mut self, f: &mut Frame, area: Rect) {
		let app = get_mut_app();
		let mut text = "? for help, q to quit".to_owned();
		if app.config.stop_key.is_some() {
			let mut keys = app.config.stop_key.as_mut().unwrap().clone();
			keys.sort();
			text += &format!(", {} to stop", keys.join(" + "));
		}
		let paragraph = Paragraph::new(text)
			.style(Style::default().fg(Color::DarkGray));
		f.render_widget(paragraph, area);
	}
}