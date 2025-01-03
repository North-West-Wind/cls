use std::{cmp::max, collections::HashSet};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{layout::Rect, style::{Color, Modifier, Style}, text::{Line, Span}, widgets::{Block, Padding, Paragraph}, Frame};
use substring::Substring;

use crate::{component::popup::{input::{AwaitInput, InputPopup}, key_bind::{KeyBindFor, KeyBindPopup}, set_popup, PopupComponent}, config, state::{config_mut, get_mut_app}};

use super::{border_style, border_type, loop_index, BlockHandleKey, BlockRenderArea};

pub struct SettingsBlock {
	id: u8,
	selected: u8,
	options: u8,
}

impl Default for SettingsBlock {
	fn default() -> Self {
		Self {
			id: 3,
			selected: 0,
			options: 4,
		}
	}
}

impl BlockRenderArea for SettingsBlock {
	fn render_area(&mut self, f: &mut Frame, area: Rect) {
		let config = config();

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
		if config.stop_key.is_empty() {
			stop_key = "".to_string();
		} else {
			let mut keys = Vec::from_iter(config.stop_key.clone().into_iter());
			keys.sort();
			stop_key = format!("{}", keys.join(" + "));
		}
		self.left_right_line("Stop Key".to_string(), stop_key, width as usize, &mut lines);
		self.left_right_line("Loopback 1".to_string(), config.loopback_1.clone(), width as usize, &mut lines);
		self.left_right_line("Loopback 2".to_string(), config.loopback_2.clone(), width as usize, &mut lines);
		self.left_right_line("Playlist Mode".to_string(), config.playlist_mode.to_string(), width as usize, &mut lines);
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
		let new_selected = loop_index(self.selected as usize, dy as i32, self.options as usize) as usize;
		let new_selected = new_selected as u8;
		if new_selected != self.selected {
			self.selected = new_selected;
			return true;
		}
		false
	}

	fn handle_enter(&mut self) -> bool {
		match self.selected {
			0 => {
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
			3 => {
				let config = config_mut();
				config.playlist_mode = !config.playlist_mode;
				return true;
			},
			_ => false
		}
	}

	fn handle_delete(&mut self) -> bool {
		let app = get_mut_app();
		let config = config_mut();
		match self.selected {
			0 => {
				config.stop_key.clear();
				app.stopkey.clear();
				return true;
			},
			1 => {
				config.loopback_1 = String::new();
				return true;
			},
			2 => {
				config.loopback_2 = String::new();
				return true;
			},
			3 => {
				config.playlist_mode = false;
				return true;
			},
			_ => false
		}
	}
}