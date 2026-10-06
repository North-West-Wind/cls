use std::{cmp::max, collections::HashSet, format, sync::{Arc, atomic::{AtomicBool, Ordering}}, thread, vec};

use crossterm::event::{KeyCode, KeyEvent};
use handy_keys::KeyboardListener;
use indexmap::IndexSet;
use parking_lot::Mutex;
use ratatui::{style::{Color, Style}, text::Line, widgets::{Block, BorderType, Clear, Padding, Paragraph, Widget}, Frame};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

use crate::{client::ClientState, common::keyboard::AnyKey};

use super::{safe_centered_rect, PopupHandleKey, PopupRender};

pub struct KeyBindPopup {
	recording: Arc<AtomicBool>,
	recorded: Arc<Mutex<IndexSet<AnyKey>>>,
	callback: Arc<Box<dyn Fn(HashSet<AnyKey>) + Send + Sync>>
}

impl KeyBindPopup {
	pub fn new(recorded: HashSet<AnyKey>, callback: impl Fn(HashSet<AnyKey>) + Send + Sync + 'static) -> Self {
		Self {
			recording: Arc::new(AtomicBool::new(false)),
			recorded: Arc::new(Mutex::new(recorded.into_iter().collect::<IndexSet<AnyKey>>())),
			callback: Arc::new(Box::new(callback))
		}
	}
}

impl PopupRender for KeyBindPopup {
	fn render(&self, f: &mut Frame) {
		let mut lines = vec![];
		lines.push(Line::from("enter: record / confirm | esc: stop | r: reset"));
		lines.push(Line::from(format!("> {}", self.recorded.lock().par_iter().map(|key| key.to_string()).collect::<Vec<String>>().join(" + "))));
		let width = max(lines[0].width(), lines[1].width()) as u16 + 4;
		let height = 4;
		let area = f.area();
		let popup_area = safe_centered_rect(width, height, area);
		Clear.render(popup_area, f.buffer_mut());
		let paragraph = Paragraph::new(lines)
			.style(if self.recording.load(Ordering::Relaxed) { Style::default().fg(Color::Yellow) } else { Style::default() })
			.block(Block::bordered().border_type(BorderType::Rounded).title("Key Bind").padding(Padding::horizontal(1)));
		f.render_widget(paragraph, popup_area);
	}
}

impl PopupHandleKey for KeyBindPopup {
	fn handle_key(&mut self, client_state: &ClientState, event: KeyEvent) -> bool {
		match event.code {
			KeyCode::Enter => {
				if !self.recording.load(Ordering::Relaxed) {
					self.recording.store(true, Ordering::Relaxed);
					let recording = self.recording.clone();
					let recorded = self.recorded.clone();
					thread::spawn(move || {
						#[cfg(target_os = "macos")]
						{
							use handy_keys::{check_accessibility, open_accessibility_settings};
							if !check_accessibility() {
								open_accessibility_settings().unwrap();
							}
						}

						let listener = KeyboardListener::new().unwrap();

						while recording.load(Ordering::Relaxed) {
							if let Ok(event) = listener.recv() && event.is_key_down {
								let mut recorded = recorded.lock();
								recorded.extend(AnyKey::split_modifiers(event.modifiers));
								if let Some(key) = event.key {
									recorded.insert(AnyKey::K(key));
								}
							}
						}
					});
				} else {
					self.recording.store(false, Ordering::Relaxed);
					let callback = self.callback.clone();
					let recorded = self.recorded.lock().par_iter().map(|key| *key).collect();
					let popups = client_state.popup_manager.clone();
					thread::spawn(move || {
						popups.pop();
						(callback)(recorded);
					});
				}
				return true;
			},
			KeyCode::Esc => {
				if self.recording.load(Ordering::Relaxed) {
					self.recording.store(false, Ordering::Relaxed);
				} else {
					self.recorded.lock().clear();
					client_state.popup_manager.pop_defer();
				}
				return true;
			},
			KeyCode::Char('r') => {
				if self.recording.load(Ordering::Relaxed) {
					return false;
				}
				self.recorded.lock().clear();
				return true;
			},
			_ => false
		}
	}
}