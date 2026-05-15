use std::{path::Path, thread};

use crossterm::event::{KeyCode, KeyEvent};
use normpath::PathExt;
use ratatui::{Frame, layout::Rect, style::{Color, Modifier, Style}, text::Line, widgets::{Block, BorderType, Clear, Padding, Paragraph, Widget}};
use substring::Substring;

use crate::{component::popup::{PopupComponent, PopupHandleKey, PopupRender, confirm::{ConfirmAction, ConfirmPopup}, defer_exit_popup, defer_set_popup, input::{FLAG_FILE, FLAG_NUM, InputPopup}, popups}, state::acquire, util::dialog::Dialog};

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
			Line::from("s - toggle sequential"),
			Line::from("enter / esc - save / discard changes"),
			Line::from(""),

			Line::from(if self.dialog.sequential { "Sequential: true".to_string() } else { format!("Delay: {} s", self.dialog.delay) }),
			Line::from(format!("Random: {}", self.dialog.random)),
		];

		let area = f.area();
		let page_size = area.height as usize - 3 - lines.len();
		let page = self.selected / page_size;
		let max_width = area.width as usize - 4;

		lines.push(Line::from(if self.dialog.files.len() > page_size {
			format!("File List (Page {} / {})", page + 1, (self.dialog.files.len() + page_size - 1) / page_size)
		} else {
			"File List".to_string()
		}).style(Style::default().add_modifier(Modifier::BOLD)).centered());

		for ii in (page * page_size)..((page + 1) * page_size).min(self.dialog.files.len()) {
			let mut file =  self.dialog.files[ii].clone();
			if file.len() > max_width {
				file = format!("{}...", file.substring(0, max_width - 3));
			}
			lines.push(Line::from(file)
				.style(if self.selected == ii {
					Style::default().fg(Color::LightGreen).add_modifier(Modifier::REVERSED)
				} else {
					Style::default().fg(Color::Green)
				}));
		}

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
			Char('s') => self.toggle_sequential(),
			Enter => self.commit_changes(),
			Esc|Char('q') => self.discard_changes(),
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
		defer_set_popup(PopupComponent::Input(InputPopup::new(String::new(), "Add Dialog File".to_string(), FLAG_FILE, |value| {
			let mut new_files= vec![];
			let path = Path::new(value);
			if path.is_dir() {
				let Ok(read_dir) = path.read_dir() else { return false; };
				read_dir.for_each(|file| {
					let Ok(entry) = file else { return; };
					let Ok(file_type) = entry.file_type() else { return; };
					if !file_type.is_dir() {
						let Ok(norm) = entry.path().normalize() else { return; };
						new_files.push(norm.clone().into_os_string().into_string().unwrap());
					}
				});
			} else {
				let Ok(norm) = path.normalize() else { return false; };
				new_files.push(norm.clone().into_os_string().into_string().unwrap());
			}
			
			thread::spawn(move || {
				let mut popups = popups();
				let Some(popup) = popups.iter_mut().find_map(|popup| {
					match popup {
						PopupComponent::Dialog(popup) => { Option::Some(popup) },
						_ => Option::None
					}
				}) else { return; };

				popup.dialog.files.append(&mut new_files);
			});
			false
		})));
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
		defer_set_popup(PopupComponent::Input(InputPopup::new(self.dialog.delay.to_string(), "Dialog Delay".to_string(), FLAG_NUM, |value| {
			let Ok(delay) = value.parse::<f32>() else { return false; };
			thread::spawn(move || {
				let mut popups = popups();
				let Some(popup) = popups.iter_mut().find_map(|popup| {
					match popup {
						PopupComponent::Dialog(popup) => { Option::Some(popup) },
						_ => Option::None
					}
				}) else { return; };

				popup.dialog.delay = delay;
			});
			false
		})));
		true
	}

	fn toggle_random(&mut self) -> bool {
		self.dialog.random = !self.dialog.random;
		true
	}

	fn toggle_sequential(&mut self) -> bool {
		self.dialog.sequential = !self.dialog.sequential;
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