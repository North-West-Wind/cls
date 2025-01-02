use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{layout::Rect, style::{Color, Style}, text::{Line, Text}, widgets::{Block, BorderType, Clear, Padding, Paragraph, Widget}, Frame};

use crate::state::{config_mut, get_mut_app};

use super::{exit_popup, PopupHandleKey, PopupRender};

pub struct DeleteTabPopup {}

impl Default for DeleteTabPopup {
	fn default() -> Self {
		Self {}
	}
}

impl PopupRender for DeleteTabPopup {
	fn render(&self, f: &mut Frame) {
		let text = Text::from(vec![
			Line::from("Press y to quit"),
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
		f.render_widget(Paragraph::new(text).block(Block::bordered().title("Delete?").padding(Padding::horizontal(1)).border_type(BorderType::Rounded).border_style(Style::default().fg(Color::Yellow))), popup_area);
	}
}

impl PopupHandleKey for DeleteTabPopup {
	fn handle_key(&mut self, event: KeyEvent) -> bool {
		match event.code {
			KeyCode::Char('y') => {
				exit_popup();
				delete_tab();
				return true
			},
			_ => {
				exit_popup();
				return false;
			}
		}
	}
}

fn delete_tab() {
	let app = get_mut_app();
	let config = config_mut();
	let selected = app.tab_selected();
	app.files.remove(&config.tabs[selected]);
	config.tabs.remove(selected);
	if selected >= config.tabs.len() && config.tabs.len() != 0 {
		app.set_tab_selected(config.tabs.len() - 1);
	}
}