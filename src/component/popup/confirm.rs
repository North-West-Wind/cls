use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{layout::Rect, style::{Color, Style}, text::{Line, Text}, widgets::{Block, BorderType, Clear, Padding, Paragraph, Widget}, Frame};

use crate::component::popup::defer_exit_popup;

use super::{PopupHandleKey, PopupRender};

type Callback = fn() -> bool;

pub struct ConfirmPopup {
	title: String,
	verb: String,
	callback: Callback,
}

impl PopupRender for ConfirmPopup {
	fn render(&self, f: &mut Frame) {
		let text = Text::from(vec![
			Line::from(format!("Press y to {}", self.verb)),
			Line::from("Press any to cancel")
		]).style(Style::default().fg(Color::Yellow));
		let width = (text.width() as u16) + 4;
		let height = (text.height() as u16) + 2;
		let area = f.area();
		let popup_area: Rect = Rect {
			x: (area.width - width) / 2,
			y: (area.height - height) / 2,
			width,
			height
		};
		Clear.render(popup_area, f.buffer_mut());
		f.render_widget(Paragraph::new(text).block(Block::bordered().title(self.title.as_str()).padding(Padding::horizontal(1)).border_type(BorderType::Rounded).border_style(Style::default().fg(Color::Yellow))), popup_area);
	}
}

impl PopupHandleKey for ConfirmPopup {
	fn handle_key(&mut self, event: KeyEvent) -> bool {
		defer_exit_popup();
		match event.code {
			KeyCode::Char('y') => (self.callback)(),
			_ => true
		}
	}
}

impl ConfirmPopup {
	pub fn new(title: &str, verb: &str, callback: Callback) -> Self {
		Self {
			title: title.to_string(),
			verb: verb.to_string(),
			callback,
		}
	}
}