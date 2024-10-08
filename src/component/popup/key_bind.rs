use std::{cmp::max, collections::{HashMap, HashSet}};

use crossterm::event::{KeyCode, KeyEvent};
use mki::Keyboard;
use ratatui::{style::{Color, Style}, text::Line, widgets::{Block, BorderType, Clear, Padding, Paragraph, Widget}, Frame};

use crate::{state::get_mut_app, util::{global_input::keyboard_to_string, notify_redraw, selected_file_path}};

use super::{exit_popup, safe_centered_rect, PopupHandleGlobalKey, PopupHandleKey, PopupRender};

pub enum KeyBindFor {
	File,
	Stop,
}

pub struct KeyBindPopup {
	this_is_a: KeyBindFor,
	recording: bool,
	recorded: HashSet<Keyboard>
}

impl KeyBindPopup {
	pub fn new(this_is_a: KeyBindFor) -> Self {
		Self {
			this_is_a,
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
		let popup_area = safe_centered_rect(width, height, area);
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
					match self.this_is_a {
						KeyBindFor::File => set_file_key_bind(&self.recorded),
						KeyBindFor::Stop => set_stop_key_bind(&self.recorded),
					}
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
		if !self.recording {
			return;
		}
		use Keyboard::*;
		match key {
			Enter|Escape => false,
			_ => self.recorded.insert(key)
		};
	}
}

fn set_file_key_bind(recorded: &HashSet<Keyboard>) {
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

fn set_stop_key_bind(recorded: &HashSet<Keyboard>) {
	let app = get_mut_app();
	app.config.stop_key = Option::Some(recorded.into_iter().map(|key| { keyboard_to_string(*key) }).collect::<Vec<String>>());
	let mut keyboard = vec![];
	for key in recorded {
		keyboard.push(*key);
	}
	app.stopkey = Option::Some(keyboard);
	notify_redraw();
}