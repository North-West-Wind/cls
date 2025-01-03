use std::path::Path;

use crossterm::event::{Event, KeyCode, KeyEvent};
use normpath::PathExt;
use ratatui::{style::{Color, Style}, widgets::{Block, BorderType, Clear, Padding, Paragraph, Widget}, Frame};
use tui_input::{backend::crossterm::EventHandler, Input};

use crate::{config::FileEntry, state::{config_mut, get_mut_app, Scanning}, util::{selected_file_path, threads::spawn_scan_thread}};

use super::{exit_popup, safe_centered_rect, PopupHandleKey, PopupHandlePaste, PopupRender};

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum AwaitInput {
	None,
	AddTab,
	Loopback1,
	Loopback2,
	SetFileId
}

pub struct InputPopup {
	input: Input,
	await_input: AwaitInput,
}

impl Default for InputPopup {
	fn default() -> Self {
		Self {
			input: Input::default(),
			await_input: AwaitInput::None,
		}
	}
}

impl InputPopup {
	pub fn new(value: String, await_input: AwaitInput) -> Self {
		Self {
			input: Input::new(value),
			await_input,
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
			.block(Block::bordered().border_type(BorderType::Rounded).title(match self.await_input {
				AwaitInput::AddTab => "Add directory as tab",
				AwaitInput::Loopback1 => "Loopback 1 (Restart)",
				AwaitInput::Loopback2 => "Loopback 2 (Restart)",
				AwaitInput::SetFileId => "File ID",
				_ => "Input"
			}).padding(Padding::horizontal(1)).style(Style::default().fg(Color::Green)));
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
			KeyCode::Enter => self.complete(&self.input, true),
			KeyCode::Esc => self.complete(&self.input, false),
			_ => {
				if self.await_input == AwaitInput::SetFileId && match event.code {
					KeyCode::Char(c) => !c.is_digit(10),
					_ => false
				} {
					return false;
				}
				
				self.input.handle_event(&Event::Key(event));
				return true;
			}
		}
		return true
	}
}

impl PopupHandlePaste for InputPopup {
	fn handle_paste(&mut self, data: String) -> bool {
		self.input = self.input.clone().with_value(self.input.value().to_owned() + data.as_str());
		return true;
	}
}

impl InputPopup {
	fn complete(&self, input: &Input, send: bool) {
		if send {
			match self.await_input {
				AwaitInput::AddTab => send_add_tab(input.value().to_string()),
				AwaitInput::Loopback1 => send_loopback_1(input.value().to_string()),
				AwaitInput::Loopback2 => send_loopback_2(input.value().to_string()),
				AwaitInput::SetFileId => send_file_id(input.value().to_string()),
				_ => (),
			}
		}
		exit_popup();
	}
}

fn send_add_tab(str: String) {
	let app = get_mut_app();
	let norm = Path::new(&str).normalize();
	if norm.is_err() {
		return;
	}
	let config = config_mut();
	config.tabs.push(norm.unwrap().into_os_string().into_string().unwrap());
	app.set_tab_selected(config.tabs.len() - 1);
	spawn_scan_thread(Scanning::One(app.tab_selected()));
}

fn send_loopback_1(str: String) {
	config_mut().loopback_1 = str;
}

fn send_loopback_2(str: String) {
	config_mut().loopback_2 = str;
}

fn send_file_id(str: String) {
	let path = selected_file_path();
	if path.is_empty() {
		return;
	}
	let id = u32::from_str_radix(&str, 10);
	if id.is_err() {
		return;
	}
	let id = id.unwrap();
	let app = get_mut_app();
	let rev_map = &mut app.rev_file_id;
	if rev_map.contains_key(&id) {
		if rev_map.get(&id).unwrap() != &path {
			app.error = "File ID must be unique".to_string();
		}
		return;
	}
	rev_map.insert(id, path.clone());
	let config = config_mut();
	match config.get_file_entry_mut(path.clone()) {
		Some(entry) => {
			entry.id = Some(id);
		},
		None => {
			let mut entry = FileEntry::default();
			entry.id = Some(id);
			config.insert_file_entry(path, entry);
		}
	}
}