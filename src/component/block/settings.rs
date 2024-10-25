use std::cmp::{max, min};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{layout::Rect, style::{Color, Modifier, Style}, text::{Line, Span}, widgets::{Block, Padding, Paragraph}, Frame};
use substring::Substring;

use crate::{component::popup::{key_bind::{KeyBindFor, KeyBindPopup}, set_popup, PopupComponent}, state::{get_app, get_mut_app}};

use super::{border_style, border_type, BlockHandleKey, BlockRenderArea};

pub struct SettingsBlock {
	id: u8,
	selected: u8,
}

impl Default for SettingsBlock {
	fn default() -> Self {
		Self {
			id: 3,
			selected: 0,
		}
	}
}

impl BlockRenderArea for SettingsBlock {
	fn render_area(&mut self, f: &mut Frame, area: Rect) {
		let app = get_app();

		let mut block = Block::bordered()
			.border_style(border_style(self.id))
			.border_type(border_type(self.id))
			.title("Settings");
		let mut width = area.width;

		if area.width > 25 {
			block = block.padding(Padding::horizontal(1));
			width -= 2;
		}

		let mut lines = vec![];
		if app.config.stop_key.is_none() {
			self.left_right_line("Stop Key".to_string(), "".to_string(), width as usize, &mut lines);
		} else {
			let mut keys = app.config.stop_key.as_ref().unwrap().clone();
			keys.sort();
			self.left_right_line("Stop Key".to_string(), format!("{}", keys.join(" + ")), width as usize, &mut lines);
		}
		f.render_widget(Paragraph::new(lines).block(block), area);
	}
}

impl BlockHandleKey for SettingsBlock {
	fn handle_key(&mut self, event: KeyEvent) -> bool {
		match event.code {
			KeyCode::Up => self.navigate_settings(-1),
			KeyCode::Down => self.navigate_settings(1),
			KeyCode::Enter => self.handle_enter(),
			KeyCode::Delete => self.handle_delete(),
			_ => false
		}
	}
}

impl SettingsBlock {
	fn left_right_line(&self, left: String, mut right: String, width: usize, lines: &mut Vec<Line>) {
		right = " ".to_owned() + right.as_str();
		let mid_span;
		if left.len() + right.len() >= width as usize {
			mid_span = Span::from("");
			right = right.substring(0, right.len() - 3).to_string() + "...";
		} else {
			mid_span = Span::from(vec![" "; max(0, width as i32 - left.len() as i32 - right.len() as i32 - 2) as usize].join(""));
		}
		let left_style;
		if lines.len() == self.selected as usize {
			left_style = Style::default().fg(Color::LightYellow).add_modifier(Modifier::REVERSED);
		} else {
			left_style = Style::default().fg(Color::LightYellow);
		}
		let left_span = Span::from(left).style(left_style);
		let right_span = Span::from(right).style(Style::default().fg(Color::Yellow));

		lines.push(Line::from(vec![left_span, mid_span, right_span]));
	}

	fn navigate_settings(&mut self, dy: i16) -> bool {
		let new_selected = min(1, max(0, self.selected as i16 + dy)) as u8;
		if new_selected != self.selected {
			self.selected = new_selected;
			return true;
		}
		false
	}

	fn handle_enter(&mut self) -> bool {
		match self.selected {
			0 => {
				set_popup(PopupComponent::KeyBind(KeyBindPopup::new(KeyBindFor::Stop)));
				return true;
			},
			_ => false
		}
	}

	fn handle_delete(&mut self) -> bool {
		match self.selected {
			0 => {
				let app = get_mut_app();
				app.config.stop_key = Option::None;
				app.stopkey = Option::None;
				return true;
			},
			_ => false
		}
	}
}