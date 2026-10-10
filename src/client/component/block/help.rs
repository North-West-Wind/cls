use std::{format, vec};

use ratatui::{Frame, layout::Rect, style::{Color, Style}, text::{Line, Span}, widgets::Paragraph};

use crate::client::ClientState;

use super::BlockRenderArea;

#[derive(Default)]
pub struct HelpBlock { }

impl BlockRenderArea for HelpBlock {
	fn render_area(&mut self, client_state: &ClientState, f: &mut Frame, area: Rect) {
		let mut spans = vec![
			Span::from("? for help, q to quit, ").style(Style::default().fg(Color::DarkGray)),
			Span::from("s to save").style(if client_state.dirty { Style::default().fg(Color::Yellow) } else { Style::default().fg(Color::DarkGray) }),
		];
		if !client_state.config.stop_key.is_empty() {
			let mut keys = client_state.config.stop_key.iter().collect::<Vec<_>>();
			keys.sort();
			let keys = keys.iter().map(|key| key.to_string()).collect::<Vec<_>>().join(" + ");
			spans.push(Span::from(format!(", {} to stop", keys)).style(Style::default().fg(Color::DarkGray)));
		}
		let paragraph = Paragraph::new(Line::from(spans))
			.style(Style::default().fg(Color::DarkGray));
		f.render_widget(paragraph, area);
	}
}