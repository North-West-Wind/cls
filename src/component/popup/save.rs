use crossterm::event::KeyEvent;
use ratatui::{layout::Rect, style::{Color, Stylize}, widgets::{Block, BorderType, Padding, Paragraph}, Frame};

use crate::component::popup::defer_exit_popup;

use super::{PopupHandleKey, PopupRender};

pub struct SavePopup {
	done: bool
}

impl SavePopup {
	pub fn new(done: bool) -> Self {
		Self {
			done
		}
	}
}

impl PopupRender for SavePopup {
	fn render(&self, f: &mut Frame) {
		let str = if self.done { "Saved!" } else { "Saving..." };
		let area = f.area();
		let width = str.len() as u16 + 4;
		let height = 3;
		let popup_area = Rect {
			x: (area.width - width) / 2,
			y: (area.height - height) / 2,
			width,
			height
		};
		f.render_widget(Paragraph::new(str).block(Block::bordered().border_type(BorderType::Thick).padding(Padding::horizontal(1))).fg(
			if self.done {
				Color::LightGreen
			} else {
				Color::Yellow
			}
		), popup_area);
	}
}

impl PopupHandleKey for SavePopup {
	fn handle_key(&mut self, _event: KeyEvent) -> bool {
		if !self.done {
			return false;
		}
		defer_exit_popup();
		return true;
	}
}