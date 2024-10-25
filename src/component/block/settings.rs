use std::cmp::{max, min};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{layout::Rect, style::{Color, Modifier, Style}, text::{Line, Span}, widgets::{Block, Padding, Paragraph}, Frame};
use substring::Substring;

use crate::{component::popup::{input::{AwaitInput, InputPopup}, key_bind::{KeyBindFor, KeyBindPopup}, set_popup, PopupComponent}, state::{get_app, get_mut_app}};

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
		let stop_key;
		if app.config.stop_key.is_none() {
			stop_key = "".to_string();
		} else {
			let mut keys = app.config.stop_key.as_ref().unwrap().clone();
			keys.sort();
			stop_key = format!("{}", keys.join(" + "));
		}
		self.left_right_line("Stop Key".to_string(), stop_key, width as usize, &mut lines);
		self.left_right_line("Loopback 1".to_string(), app.config.loopback_1.clone(), width as usize, &mut lines);
		self.left_right_line("Loopback 2".to_string(), app.config.loopback_2.clone(), width as usize, &mut lines);
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
		let new_selected = min(2, max(0, self.selected as i16 + dy)) as u8;
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
				set_popup(PopupComponent::KeyBind(KeyBindPopup::new(KeyBindFor::Stop, HashSet::new())));
				return true;
			},
			1|2 => {
				let await_input;
				if self.selected == 1 {
					await_input = AwaitInput::Loopback1;
				} else {
					await_input = AwaitInput::Loopback2;
				}
				set_popup(PopupComponent::Input(InputPopup::new(String::new(), await_input)));
				return true;
			},
			_ => false
		}
	}

	fn handle_delete(&mut self) -> bool {
		let app = get_mut_app();
		match self.selected {
			0 => {
				app.config.stop_key = Option::None;
				app.stopkey = Option::None;
				return true;
			},
			1 => {
				app.config.loopback_1 = String::new();
				return true;
			},
			2 => {
				app.config.loopback_2 = String::new();
				return true;
			},
			_ => false
		}
	}
}