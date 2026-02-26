use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{Frame, layout::Rect, style::{Color, Modifier, Style}, text::Line, widgets::{Block, BorderType, Clear, Padding, Paragraph, Widget}};

use crate::{component::popup::{PopupComponent, PopupHandleKey, PopupRender, confirm::{ConfirmAction, ConfirmPopup}, defer_exit_popup, defer_set_popup, input::{AwaitInput, InputPopup}}, state::acquire, util::dialog::Dialog};

pub struct DialogPopup {
	index: usize,
	pub(super) dialog: Dialog,
	pub(super) selected: usize,
	pub(super) changed: bool
}

impl DialogPopup {
	pub fn new(index: usize) -> Self {
		Self {
			index,
			dialog: acquire().dialogs[index].clone(),
			selected: 0,
			changed: false
		}
	}
}

impl PopupRender for DialogPopup {
	fn render(&self, f: &mut Frame) {
		let mut lines = vec![
			Line::from("Controls").style(Style::default().add_modifier(Modifier::BOLD)).centered(),
			Line::from("a - add, d - delete"),
			Line::from("up / down - select"),
			Line::from("c - change delay, r - toggle random"),
			Line::from("enter / esc - save / discard changes"),
			Line::from(""),

			Line::from(format!("Delay: {} s", self.dialog.delay)),
			Line::from(format!("Random: {}", self.dialog.random)),
			Line::from("File List").style(Style::default().add_modifier(Modifier::BOLD)).centered()
		];

		for (ii, file) in self.dialog.files.iter().enumerate() {
			lines.push(Line::from(file.clone())
				.style(if self.selected == ii {
					Style::default().fg(Color::LightGreen).add_modifier(Modifier::REVERSED)
				} else {
					Style::default().fg(Color::Green)
				}));
		}

		let area = f.area();
		let width = lines.iter()
			.map(|line| { line.width() })
			.fold(0, |acc, width| acc.max(width)) as u16 + 4;
		let height = lines.len() as u16 + 2;

		let popup_area = Rect {
			x: (area.width - width) / 2,
			y: (area.height - height) / 2,
			width,
			height
		};

		let block = Block::bordered()
			.padding(Padding::horizontal(1))
			.border_type(BorderType::Rounded)
			.title("Editor");

		Clear.render(popup_area, f.buffer_mut());
		f.render_widget(Paragraph::new(lines).block(block), popup_area);
	}
}

impl PopupHandleKey for DialogPopup {
	fn handle_key(&mut self, event: KeyEvent) -> bool {
		use KeyCode::*;
		match event.code {
			Up => self.navigate_files(-1),
			Down => self.navigate_files(1),
			Char('a') => self.add_file(),
			Char('d') => self.delete_file(),
			Char('c') => self.change_delay(),
			Char('r') => self.toggle_random(),
			Enter => self.commit_changes(),
			Esc => self.discard_changes(),
			_ => false
		}
	}
}

impl DialogPopup {
	fn navigate_files(&mut self, dy: i16) -> bool {
		let changed = self.selected as i16 + dy;
		let new_selected: usize;
		if changed < 0 {
			new_selected = self.dialog.files.len() - 1;
		} else if changed as usize >= self.dialog.files.len() {
			new_selected = 0;
		} else {
			new_selected = changed as usize;
		}
		if new_selected != self.selected {
			self.selected = new_selected;
			return true;
		}
		false
	}

	fn add_file(&mut self) -> bool {
		defer_set_popup(PopupComponent::Input(InputPopup::new(String::new(), AwaitInput::AddDialogFile)));
		true
	}

	fn delete_file(&mut self) -> bool {
		if self.dialog.files.len() <= 1 {
			return false;
		}
		self.dialog.files.remove(self.selected);
		if self.selected >= self.dialog.files.len() {
			self.selected = self.dialog.files.len() - 1;
		}
		true
	}

	fn change_delay(&self) -> bool {
		defer_set_popup(PopupComponent::Input(InputPopup::new(self.dialog.delay.to_string(), AwaitInput::DialogDelay)));
		true
	}

	fn toggle_random(&mut self) -> bool {
		self.dialog.random = !self.dialog.random;
		true
	}

	fn commit_changes(&self) -> bool {
		let mut app = acquire();
		app.dialogs[self.index] = self.dialog.clone();
		app.config.dialogs[self.index] = self.dialog.to_entry();
		defer_exit_popup();
		true
	}

	fn discard_changes(&self) -> bool {
		if self.changed {
			defer_set_popup(PopupComponent::Confirm(ConfirmPopup::new(ConfirmAction::DiscardDialogChanges)));
		} else {
			defer_exit_popup();
		}
		true
	}
}