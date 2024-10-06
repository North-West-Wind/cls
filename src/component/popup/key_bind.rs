use std::{cmp::max, collections::{HashMap, HashSet}};

use crossterm::event::{KeyCode, KeyEvent};
use mki::Keyboard;
use ratatui::{layout::Rect, style::{Color, Style}, text::Line, widgets::{Block, BorderType, Clear, Padding, Paragraph, Widget}, Frame};

use crate::{util::global_input::keyboard_to_string, state::get_mut_app, util::selected_file_path};

use super::{exit_popup, PopupHandleGlobalKey, PopupHandleKey, PopupRender};

pub struct KeyBindPopup {
	recording: bool,
	recorded: HashSet<Keyboard>
}

impl Default for KeyBindPopup {
	fn default() -> Self {
		Self {
			recording: false,
			recorded: HashSet::new(),
		}
	}
}

impl PopupRender for KeyBindPopup {
	fn render(&self, f: &mut Frame) {
		let mut lines = vec![];
		lines.push(Line::from("enter: record / confirm | esc: stop | r: reset"));
		lines.push(Line::from(format!("> {}", self.recorded.clone().into_iter().map(|key| { keyboard_to_string(key) }).collect::<Vec<String>>().join(" + "))));
		let width = max(lines[0].width(), lines[1].width()) as u16 + 4;
		let height = 4;
		let area = f.area();
		let popup_area = Rect {
			x: (area.width - width) / 2,
			y: (area.height - height) / 2,
			width,
			height
		};
		Clear.render(popup_area, f.buffer_mut());
		let paragraph = Paragraph::new(lines)
			.style(if self.recording { Style::default().fg(Color::Yellow) } else { Style::default() })
			.block(Block::bordered().border_type(BorderType::Rounded).title("Key Bind").padding(Padding::horizontal(1)));
		f.render_widget(paragraph, popup_area);
	}
}

impl PopupHandleKey for KeyBindPopup {
	fn handle_key(&mut self, event: KeyEvent) -> bool {
		match event.code {
			KeyCode::Enter => {
				if !self.recording {
					self.recording = true;
				} else {
					self.recording = false;
					set_key_bind(&self.recorded);
					exit_popup();
				}
				return true;
			},
			KeyCode::Esc => {
				if self.recording {
					self.recording = false;
				} else {
					self.recorded.clear();
					exit_popup();
				}
				return true;
			},
			KeyCode::Char('r') => {
				if self.recording {
					return false;
				}
				self.recorded.clear();
				return true;
			},
			_ => false
		}
	}
}

impl PopupHandleGlobalKey for KeyBindPopup {
	fn handle_global_key(&mut self, key: Keyboard) {
		use Keyboard::*;
		match key {
			Enter|Escape => false,
			_ => self.recorded.insert(key)
		};
	}
}

fn set_key_bind(recorded: &HashSet<Keyboard>) {
	let path = selected_file_path();
	if path.is_empty() {
		return;
	}
	let app = get_mut_app();
	if app.config.file_key.is_none() {
		app.config.file_key = Option::Some(HashMap::new());
	}
	let map = app.config.file_key.as_mut().unwrap();
	map.insert(path.clone(), recorded.into_iter().map(|key| { keyboard_to_string(*key) }).collect::<Vec<String>>());
	let mut keyboard = vec![];
	for key in recorded {
		keyboard.push(*key);
	}
	app.hotkey.as_mut().unwrap().insert(path, keyboard);
}