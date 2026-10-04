use std::{sync::Arc, thread};

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{layout::Rect, style::{Color, Style}, text::{Line, Text}, widgets::{Block, BorderType, Clear, Padding, Paragraph, Widget}, Frame};

use crate::client::ClientState;

use super::{PopupHandleKey, PopupRender};

pub struct ConfirmPopup {
	title: String,
	verb: String,
	callback: Arc<Box<dyn Fn() + Send + Sync>>,
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
	fn handle_key(&mut self, client_state: &ClientState, event: KeyEvent) -> bool {
		match event.code {
			KeyCode::Char('y') => {
				let callback = self.callback.clone();
				let popups = client_state.popup_manager.clone();
				thread::spawn(move || {
					popups.pop();
					(callback)();
				});
			},
			_ => {
				client_state.popup_manager.pop();
			}
		}
		true
	}
}

impl ConfirmPopup {
	pub fn new(title: &str, verb: &str, callback: impl Fn() + Send + Sync + 'static) -> Self {
		Self {
			title: title.to_string(),
			verb: verb.to_string(),
			callback: Arc::new(Box::new(callback)),
		}
	}
}