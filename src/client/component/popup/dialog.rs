use std::{format, path::Path, sync::Arc, thread, vec};

use crossterm::event::{KeyCode, KeyEvent};
use normpath::PathExt;
use ratatui::{Frame, layout::Rect, style::{Color, Modifier, Style}, text::Line, widgets::{Block, BorderType, Clear, Padding, Paragraph, Widget}};
use rayon::iter::{IndexedParallelIterator, IntoParallelRefIterator, ParallelIterator};
use substring::Substring;

use crate::{client::{ClientState, component::popup::{PopupComponent, PopupHandleKey, PopupRender, confirm::ConfirmPopup, input::{FLAG_FILE, FLAG_NUM, InputPopup}}}, common::base::dialog::Dialog};

pub struct DialogPopup {
	pub(super) dialog: Dialog,
	pub(super) selected: usize,
	pub(super) changed: bool,
	pub(super) on_commit: Arc<Box<dyn Fn(Dialog) + Send + Sync>>
}

impl DialogPopup {
	pub fn new(dialog: Dialog, on_commit: impl Fn(Dialog) + Send + Sync + 'static) -> Self {
		Self {
			dialog: dialog.clone(),
			selected: 0,
			changed: false,
			on_commit: Arc::new(Box::new(on_commit))
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

		lines.extend(self.dialog.files[page * page_size..((page + 1) * page_size).min(self.dialog.files.len())].par_iter().enumerate().map(|(ii, file)| {
			let file = if file.len() > max_width {
				format!("{}...", file.substring(0, max_width - 3))
			} else {
				file.clone()
			};
			Line::from(file).style(if self.selected == ii {
				Style::default().fg(Color::LightGreen).add_modifier(Modifier::REVERSED)
			} else {
				Style::default().fg(Color::Green)
			})
		}).collect::<Vec<_>>());

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

		let width = lines.par_iter().map(|line| { line.width() as u16 }).max().unwrap() + 4;
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
	fn handle_key(&mut self, client_state: &ClientState, event: KeyEvent) -> bool {
		use KeyCode::*;
		match event.code {
			Up => self.navigate_files(-1),
			Down => self.navigate_files(1),
			Char('a') => self.add_file(client_state),
			Char('d') => self.delete_file(),
			Char('c') => self.change_delay(client_state),
			Char('r') => self.toggle_random(),
			Char('s') => self.toggle_sequential(),
			Enter => self.commit_changes(client_state),
			Esc|Char('q') => self.discard_changes(client_state),
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

	fn add_file(&mut self, client_state: &ClientState) -> bool {
		let popup_manager = client_state.popup_manager.clone();
		client_state.popup_manager.push(PopupComponent::Input(InputPopup::new(String::new(), "Add Dialog File".to_string(), FLAG_FILE, move |value| {
			let mut new_files= vec![];
			let path = Path::new(value);
			if path.is_dir() {
				let Ok(read_dir) = path.read_dir() else { return; };
				read_dir.for_each(|file| {
					let Ok(entry) = file else { return; };
					let Ok(file_type) = entry.file_type() else { return; };
					if !file_type.is_dir() {
						let Ok(norm) = entry.path().normalize() else { return; };
						new_files.push(norm.clone().into_os_string().into_string().unwrap());
					}
				});
			} else {
				let Ok(norm) = path.normalize() else { return; };
				new_files.push(norm.clone().into_os_string().into_string().unwrap());
			}

			let mut popups = popup_manager.popups.lock();
			if let Some(PopupComponent::Dialog(popup)) = popups.last_mut() {
				popup.dialog.files.extend(new_files);
				popup.changed = true;
			}
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

	fn change_delay(&self, client_state: &ClientState) -> bool {
		let popup_manager = client_state.popup_manager.clone();
		client_state.popup_manager.push(PopupComponent::Input(InputPopup::new(self.dialog.delay.to_string(), "Dialog Delay".to_string(), FLAG_NUM, move |value| {
			let Ok(delay) = value.parse::<f32>() else { return; };
			let mut popups = popup_manager.popups.lock();
			if let Some(PopupComponent::Dialog(popup)) = popups.last_mut() {
				popup.dialog.delay = delay;
				popup.changed = true;
			}
		})));
		true
	}

	fn toggle_random(&mut self) -> bool {
		self.dialog.random = !self.dialog.random;
		self.changed = true;
		true
	}

	fn toggle_sequential(&mut self) -> bool {
		self.dialog.sequential = !self.dialog.sequential;
		self.changed = true;
		true
	}

	fn commit_changes(&self, client_state: &ClientState) -> bool {
		let callback = self.on_commit.clone();
		let dialog = self.dialog.clone();
		let popups = client_state.popup_manager.clone();
		thread::spawn(move || {
			popups.pop();
			(callback)(dialog);
		});
		false
	}

	fn discard_changes(&self, client_state: &ClientState) -> bool {
		if self.changed {
			let popup_manager = client_state.popup_manager.clone();
			client_state.popup_manager.push(PopupComponent::Confirm(ConfirmPopup::new("Discard changes?", "discard", move || { popup_manager.pop(); })));
		} else {
			client_state.popup_manager.pop_defer();
		}
		true
	}
}