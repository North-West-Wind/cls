use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{layout::Rect, style::{Color, Style}, text::{Line, Text}, widgets::{Block, BorderType, Clear, Padding, Paragraph, Widget}, Frame};

use crate::{component::{block::{BlockSingleton, dialogs::DialogBlock, tabs::TabsBlock, waves::WavesBlock}, popup::defer_exit_popup}, state::{acquire, acquire_running}};

use super::{PopupHandleKey, PopupRender};

pub enum ConfirmAction {
	DeleteTab,
	DeleteWave,
	DeleteDialog,
	DiscardWaveChanges,
	DiscardDialogChanges,
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
		defer_exit_popup();
		match event.code {
			KeyCode::Char('y') => {
				match self.action {
					DeleteTab => self.delete_tab(),
					DeleteWave => self.delete_wave(),
					DeleteDialog => self.delete_dialog(),
					DiscardWaveChanges|DiscardDialogChanges => self.exit_popup(),
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
			DeleteTab|DeleteWave|DeleteDialog => {
				title = "Delete?";
				verb = "delete";
			},
			DiscardWaveChanges|DiscardDialogChanges => {
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
		let mut app = acquire();
		let mut tabs_block = TabsBlock::instance();
		let selected = tabs_block.selected;
		let tab = app.config.tabs[selected].clone();
		app.files.remove(&tab);
		app.config.tabs.remove(selected);
		let len = app.config.tabs.len();
		if selected >= len && len != 0 {
			tabs_block.selected = len - 1;
		}
		true
	}

	fn delete_wave(&self) -> bool {
		let mut app = acquire();
		let mut wave_block = WavesBlock::instance();
		let selected = wave_block.selected;
		app.waves.remove(selected);
		app.config.waves.remove(selected);
		let len = app.config.waves.len();
		if selected >= len && len != 0 {
			wave_block.selected = len - 1;
		}
		true
	}

	fn delete_dialog(&self) -> bool {
		let mut app = acquire();
		let mut dialog_block = DialogBlock::instance();
		let selected = dialog_block.selected;
		app.dialogs.remove(selected);
		app.config.dialogs.remove(selected);
		let len = app.config.dialogs.len();
		if selected >= len && len != 0 {
			dialog_block.selected = len - 1;
		}
		true
	}

	fn exit_popup(&self) -> bool {
		defer_exit_popup(); // exit the wave popup
		true
	}

	fn quit(&self) -> bool {
		*acquire_running() = false;
		false
	}
}