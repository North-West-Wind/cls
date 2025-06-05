use std::{cmp::max, collections::HashSet};

use crossterm::event::{KeyCode, KeyEvent};
use mki::Keyboard;
use ratatui::{style::{Color, Style}, text::Line, widgets::{Block, BorderType, Clear, Padding, Paragraph, Widget}, Frame};

use crate::{config::FileEntry, state::{config_mut, get_mut_app}, util::{global_input::keyboard_to_string, notify_redraw, selected_file_path}};

use super::{exit_popup, safe_centered_rect, PopupHandleGlobalKey, PopupHandleKey, PopupRender};

pub enum KeyBindFor {
	File,
	Stop,
	Wave,
}

pub struct KeyBindPopup {
	this_is_a: KeyBindFor,
	recording: bool,
	recorded: HashSet<Keyboard>
}

impl KeyBindPopup {
	pub fn new(this_is_a: KeyBindFor, recorded: HashSet<Keyboard>) -> Self {
		Self {
			this_is_a,
			recording: false,
			recorded: recorded,
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
						KeyBindFor::File => self.set_file_key_bind(),
						KeyBindFor::Stop => self.set_stop_key_bind(),
						KeyBindFor::Wave => self.set_wave_key_bind(),
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

impl KeyBindPopup {
	fn set_file_key_bind(&self) {
		let path = selected_file_path();
		if path.is_empty() {
			return;
		}
		let app = get_mut_app();
		let config = config_mut();
		match config.get_file_entry_mut(path.clone()) {
			Some(entry) => {
				entry.keys = self.recorded.iter().map(|key| { keyboard_to_string(*key) }).collect::<HashSet<String>>();
			},
			None => {
				let mut entry = FileEntry::default();
				entry.keys = self.recorded.iter().map(|key| { keyboard_to_string(*key) }).collect::<HashSet<String>>();
				config.insert_file_entry(path.clone(), entry);
			}
		}
		app.hotkey.insert(path, self.recorded.clone().into_iter().collect::<Vec<Keyboard>>());
	}

	fn set_stop_key_bind(&self) {
		let app = get_mut_app();
		config_mut().stop_key = self.recorded.iter().map(|key| { keyboard_to_string(*key) }).collect::<HashSet<String>>();
		app.stopkey = self.recorded.clone().into_iter().collect::<Vec<Keyboard>>();
		notify_redraw();
	}

	fn set_wave_key_bind(&self) {
		let app = get_mut_app();
		let selected = app.wave_selected();
		config_mut().waves[selected].keys = self.recorded.iter().map(|key| { keyboard_to_string(*key) }).collect::<HashSet<String>>();
		app.waves[selected].keys = self.recorded.clone().into_iter().collect::<Vec<Keyboard>>();
		notify_redraw();
	}
}