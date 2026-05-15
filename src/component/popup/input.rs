use std::{fs, path::Path};

use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::{style::{Color, Style}, widgets::{Block, BorderType, Clear, Padding, Paragraph, Widget}, Frame};
use tui_input::{Input, InputRequest, backend::crossterm::EventHandler};

use crate::{component::{popup::defer_exit_popup}};

use super::{safe_centered_rect, PopupHandleKey, PopupHandlePaste, PopupRender};

type Callback = fn(value: &str) -> bool;

pub const FLAG_NONE: u8 = 0;
pub const FLAG_NUM: u8 = 1;
pub const FLAG_INT: u8 = 2;
pub const FLAG_DIR: u8 = 4;
pub const FLAG_FILE: u8 = 8;

pub struct InputPopup {
	input: Input,
	title: String,
	flags: u8,
	callback: Callback,
}

impl InputPopup {
	pub fn new(value: String, title: String, flags: u8, callback: Callback) -> Self {
		Self {
			input: Input::new(value),
			title,
			flags,
			callback
		}
	}
}

impl PopupRender for InputPopup {
	fn render(&self, f: &mut Frame) {
		let area = f.area();
		let width = (area.width / 2).max(5);
		let height = 3;
		let input = &self.input;
		let scroll = input.visual_scroll(width as usize - 5);
		let input_para = Paragraph::new(input.value())
			.scroll((0, scroll as u16))
			.block(Block::bordered().border_type(BorderType::Rounded).border_style(Style::default().fg(Color::Green)).title(self.title.clone()).padding(Padding::horizontal(1)).style(Style::default().fg(Color::Cyan)));
		let input_area = safe_centered_rect(width, height, area);
		Clear.render(input_area, f.buffer_mut());
		f.render_widget(input_para, input_area);
		f.set_cursor_position((
			input_area.x + ((input.visual_cursor()).max(scroll) - scroll) as u16 + 2,
			input_area.y + 1
		));
	}
}

impl PopupHandleKey for InputPopup {
	fn handle_key(&mut self, event: KeyEvent) -> bool {
		match event.code {
			KeyCode::Enter => self.complete(true),
			KeyCode::Esc => self.complete(false),
			KeyCode::Char(c) => {
				if c == 'h' && event.modifiers == KeyModifiers::CONTROL {
					// ctrl+backspace is parsed as ctrl+h
					self.input.handle(InputRequest::DeletePrevWord);
					return true;
				} else if self.flags & FLAG_INT != 0 && !c.is_digit(10) {
					return false;
				} else if self.flags & FLAG_NUM != 0 {
					let new = format!("{}{}", self.input.value(), c);
					if let Err(_) = new.parse::<f32>() {
						return false;
					}
				}
				self.input.handle_event(&Event::Key(event));
				true
			},
			KeyCode::Delete => {
				if event.modifiers == KeyModifiers::CONTROL {
					self.input.handle(InputRequest::DeleteNextWord);
				} else {
					self.input.handle_event(&Event::Key(event));
				}
				true
			},
			KeyCode::Tab => {
				if self.flags & FLAG_FILE != 0 || self.flags & FLAG_DIR != 0 {
					// Try to auto-fill path
					let input = self.input.value();
					let parent: &Path;
					let file_name: String;
					let path = Path::new(input);
					if input.ends_with("/") {
						parent = path;
						file_name = String::new();
					} else {
						let Some(path_parent) = path.parent() else { return false };
						parent = path_parent;
						let path_file_name = path.file_name();
						if let Some(os_str) = path_file_name {
							if let Some(str) = os_str.to_str() {
								file_name = str.to_string();
							} else {
								file_name = String::new();
							}
						} else {
							file_name = String::new();
						}
					}
					let Ok(files) = parent.read_dir() else { return false };
					let mut file_names = vec![];
					for file in files {
						let Ok(file) = file else { continue };
						if self.flags & FLAG_DIR != 0 {
							let Ok(file_type) = file.file_type() else { continue };
							if file_type.is_symlink() {
								let result = fs::read_link(path);
								if result.is_err() || !Path::is_dir(Path::new(&result.unwrap())) { continue; }
							} else if !file_type.is_dir() { continue; }
						}
						let os_str = file.file_name();
						let Some(file) = os_str.to_str() else { continue };
						file_names.push(file.to_string());
					}
					if file_name.is_empty() {
						if file_names.len() == 1 {
							let joined = parent.join(file_names[0].clone());
							self.input = self.input.clone().with_value(joined.to_str().unwrap().to_string());
						} else {
							return false;
						}
					} else {
						let mut prefix = String::new();
						for file in file_names {
							if file.starts_with(&file_name) {
								if prefix.is_empty() {
									prefix = file;
								} else {
									// Get common prefix
									while !file_name.starts_with(&prefix) {
										if prefix.is_empty() {
											prefix = String::new();
											break;
										}
										prefix.pop();
									}
								}
							}
						}
						if prefix.is_empty() {
							return false;
						}
						let joined = parent.join(prefix);
						let new_input = if Path::is_dir(&joined) {
							format!("{}/", joined.to_str().unwrap())
						} else {
							joined.to_str().unwrap().to_string()
						};
						self.input = self.input.clone().with_value(new_input);
					}
				}
				true
			},
			_ => {
				self.input.handle_event(&Event::Key(event));
				true
			}
		}
	}
}

impl PopupHandlePaste for InputPopup {
	fn handle_paste(&mut self, data: String) -> bool {
		self.input = self.input.clone().with_value(self.input.value().to_owned() + data.as_str());
		return true;
	}
}

impl InputPopup {
	fn complete(&self, send: bool) -> bool {
		let redraw = if send {
			(self.callback)(self.input.value())
		} else {
			false
		};
		defer_exit_popup();
		redraw
	}
}