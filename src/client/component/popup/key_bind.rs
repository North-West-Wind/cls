use std::{cmp::max, collections::HashSet, format, sync::Arc, thread, vec};

use crossterm::event::{KeyCode, KeyEvent};
use indexmap::IndexSet;
use mki::Keyboard;
use ratatui::{style::{Color, Style}, text::Line, widgets::{Block, BorderType, Clear, Padding, Paragraph, Widget}, Frame};

use crate::{client::ClientState, common::keyboard::keyboard_to_string};

use super::{safe_centered_rect, PopupHandleGlobalKey, PopupHandleKey, PopupRender};

pub struct KeyBindPopup {
	recording: bool,
	recorded: IndexSet<Keyboard>,
	callback: Arc<Box<dyn Fn(HashSet<Keyboard>) + Send + Sync>>
}

impl KeyBindPopup {
	pub fn new(recorded: HashSet<Keyboard>, callback: impl Fn(HashSet<Keyboard>) + Send + Sync + 'static) -> Self {
		Self {
			recording: false,
			recorded: recorded.into_iter().collect::<IndexSet<Keyboard>>(),
			callback: Arc::new(Box::new(callback))
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
	fn handle_key(&mut self, client_state: &ClientState, event: KeyEvent) -> bool {
		match event.code {
			KeyCode::Enter => {
				if !self.recording {
					self.recording = true;
				} else {
					self.recording = false;
					let callback = self.callback.clone();
					let recorded = self.recorded.iter().map(|key| *key).collect();
					let popups = client_state.popup_manager.clone();
					thread::spawn(move || {
						popups.pop();
						(callback)(recorded);
					});
				}
				return true;
			},
			KeyCode::Esc => {
				if self.recording {
					self.recording = false;
				} else {
					self.recorded.clear();
					client_state.popup_manager.pop_defer();
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