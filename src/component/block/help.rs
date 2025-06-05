use ratatui::{layout::Rect, style::{Color, Style}, widgets::Paragraph, Frame};

use crate::{state::config, util::global_input::sort_keys};

use super::BlockRenderArea;

pub struct HelpBlock { }

impl Default for HelpBlock {
	fn default() -> Self {
		Self { }
	}
}

impl BlockRenderArea for HelpBlock {
	fn render_area(&mut self, f: &mut Frame, area: Rect) {
		let config = config();
		let mut text = "? for help, q to quit".to_owned();
		if !config.stop_key.is_empty() {
			let mut keys = config.stop_key.clone().into_iter().collect::<Vec<String>>();
			let keys = sort_keys(&mut keys);
			text += &format!(", {} to stop", keys.join(" + "));
		}
		let paragraph = Paragraph::new(text)
			.style(Style::default().fg(Color::DarkGray));
		f.render_widget(paragraph, area);
	}
}