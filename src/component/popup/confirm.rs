use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{layout::Rect, style::{Color, Style}, text::{Line, Text}, widgets::{Block, BorderType, Clear, Padding, Paragraph, Widget}, Frame};

use crate::state::{config_mut, get_mut_app};

use super::{exit_popup, PopupHandleKey, PopupRender};

pub enum ConfirmAction {
	DeleteTab,
	DeleteWave,
	DiscardWaveChanges,
	Quit
}

pub struct ConfirmPopup {
	title: String,
	verb: String,
	action: ConfirmAction
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
		use ConfirmAction::*;
		exit_popup();
		match event.code {
			KeyCode::Char('y') => {
				match self.action {
					DeleteTab => self.delete_tab(),
					DeleteWave => self.delete_wave(),
					DiscardWaveChanges => self.discard_wave_changes(),
					Quit => self.quit()
				}
			},
			_ => true
		}
	}
}

impl ConfirmPopup {
	pub fn new(action: ConfirmAction) -> Self {
		use ConfirmAction::*;
		let title: &str;
		let verb: &str;
		match action {
			DeleteTab|DeleteWave => {
				title = "Delete?";
				verb = "delete";
			},
			DiscardWaveChanges => {
				title = "Discard?";
				verb = "discard";
			},
			Quit => {
				title = "Quit?";
				verb = "quit";
			}
		}
		Self {
			title: title.to_string(),
			verb: verb.to_string(),
			action
		}
	}

	fn delete_tab(&self) -> bool {
		let app = get_mut_app();
		let config = config_mut();
		let selected = app.tab_selected();
		app.files.remove(&config.tabs[selected]);
		config.tabs.remove(selected);
		if selected >= config.tabs.len() && config.tabs.len() != 0 {
			app.set_tab_selected(config.tabs.len() - 1);
		}
		true
	}

	fn delete_wave(&self) -> bool {
		let app = get_mut_app();
		let config = config_mut();
		let selected = app.wave_selected();
		app.waves.remove(selected);
		config.waves.remove(selected);
		if selected >= config.waves.len() && config.waves.len() != 0 {
			app.set_wave_selected(config.waves.len() - 1);
		}
		true
	}

	fn discard_wave_changes(&self) -> bool {
		exit_popup(); // exit the wave popup
		true
	}

	fn quit(&self) -> bool {
		let app = get_mut_app();
		app.running = false;
		false
	}
}