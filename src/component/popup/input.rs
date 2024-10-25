use std::path::Path;

use crossterm::event::{Event, KeyCode, KeyEvent};
use normpath::PathExt;
use ratatui::{style::{Color, Style}, widgets::{Block, BorderType, Clear, Padding, Paragraph, Widget}, Frame};
use tui_input::{backend::crossterm::EventHandler, Input};

use crate::{state::{get_app, get_mut_app, AwaitInput, Scanning}, util::threads::spawn_scan_thread};

use super::{exit_popup, safe_centered_rect, PopupHandleKey, PopupHandlePaste, PopupRender};

pub struct InputPopup {
	input: Input,
}

impl Default for InputPopup {
	fn default() -> Self {
		Self {
			input: Input::default()
		}
	}
}

impl InputPopup {
	pub fn new(value: String) -> Self {
		Self {
			input: Input::new(value)
		}
	}
}

impl PopupRender for InputPopup {
	fn render(&self, f: &mut Frame) {
		let app = get_app();
		let area = f.area();
		let width = (area.width / 2).max(5);
		let height = 3;
		let input = &self.input;
		let scroll = input.visual_scroll(width as usize - 5);
		let input_para = Paragraph::new(input.value())
			.scroll((0, scroll as u16))
			.block(Block::bordered().border_type(BorderType::Rounded).title(match app.await_input {
				AwaitInput::AddTab => "Add directory as tab",
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
			KeyCode::Enter => complete(&self.input, true),
			KeyCode::Esc => complete(&self.input, false),
			_ => {
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

fn complete(input: &Input, send: bool) {
	let app = get_mut_app();
	if send {
		match app.await_input {
			AwaitInput::AddTab => send_add_tab(input.value().to_string()),
			_ => (),
		}
	}
	app.await_input = AwaitInput::None;
	exit_popup();
}

fn send_add_tab(str: String) {
	let app = get_mut_app();
	let norm = Path::new(&str).normalize();
	if norm.is_err() {
		return;
	}
	app.config.tabs.push(norm.unwrap().into_os_string().into_string().unwrap());
	app.set_tab_selected(app.config.tabs.len() - 1);
	spawn_scan_thread(Scanning::One(app.tab_selected()));
}