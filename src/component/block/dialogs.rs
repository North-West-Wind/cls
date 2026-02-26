use std::{collections::HashSet, sync::{Mutex, MutexGuard, OnceLock}};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use mki::Keyboard;
use rand::Rng;
use ratatui::{Frame, layout::Rect, style::{Color, Modifier, Style}, text::{Line, Span}, widgets::{Block, Borders, Padding, Paragraph}};

use crate::{component::{block::{BlockHandleKey, BlockNavigation, BlockRenderArea, BlockSingleton, loop_index, settings::SettingsBlock, tabs::TabsBlock}, popup::{PopupComponent, confirm::{ConfirmAction, ConfirmPopup}, dialog::DialogPopup, input::{AwaitInput, InputPopup}, key_bind::{KeyBindFor, KeyBindPopup}, set_popup}}, state::acquire, util::dialog::Dialog};

pub struct DialogBlock {
	range: (i32, i32),
	height: u16,
	pub selected: usize,
}

impl BlockSingleton for DialogBlock {
	fn instance() -> MutexGuard<'static, Self> {
		static BLOCK: OnceLock<Mutex<DialogBlock>> = OnceLock::new();
		BLOCK.get_or_init(|| {
			Mutex::new(Self {
				range: (-1, -1),
				height: 0,
				selected: 0,
			})
		}).lock().unwrap()
	}
}

impl BlockRenderArea for DialogBlock {
	fn render_area(&mut self, f: &mut Frame, area: Rect) {
		if self.range.0 == -1 || self.height != area.height {
			self.range = (0, area.height as i32 - 5);
			self.height = area.height;
		}

		let app = acquire();
		let (border_type, border_style) = app.borders(Self::ID);
		let block = Block::default()
			.title("Dialog")
			.borders(Borders::ALL)
			.border_type(border_type)
			.border_style(border_style.fg(Color::Yellow))
			.padding(Padding::new(2, 2, 1, 1));

		let paragraph: Paragraph;
		if app.dialogs.len() == 0 {
			paragraph = Paragraph::new("Add a dialog to get started! :>");
		} else {
			let mut lines = vec![];
			for (ii, dialog) in app.dialogs.iter().enumerate() {
				let mut spans = vec![];
				if dialog.id.is_some() {
					spans.push(Span::from("I").style(Style::default().fg(Color::LightYellow).add_modifier(Modifier::REVERSED)));
				} else {
					spans.push(Span::from(" "));
				}
				if !dialog.keys.is_empty() {
					spans.push(Span::from("K").style(Style::default().fg(Color::LightGreen).add_modifier(Modifier::REVERSED)));
				} else {
					spans.push(Span::from(" "));
				}
				spans.push(Span::from(" "));
				let style = if self.selected == ii {
					Style::default().fg(Color::LightYellow).add_modifier(Modifier::REVERSED)
				} else {
					Style::default().fg(Color::Yellow)
				};
				lines.push(Line::from(Span::from(dialog.label.clone()).style(style)));
			}
			if self.selected < self.range.0 as usize {
				self.range = (self.selected as i32, self.selected as i32 + area.height as i32 - 5);
			} else if self.selected > self.range.1 as usize {
				self.range = (self.selected as i32 - area.height as i32 + 5, self.selected as i32);
			}
			paragraph = Paragraph::new(lines).scroll((self.range.0 as u16, 0));
		}

		f.render_widget(paragraph.block(block), area);
	}
}

impl BlockHandleKey for DialogBlock {
	fn handle_key(&mut self, event: KeyEvent) -> bool {
		if event.modifiers.contains(KeyModifiers::CONTROL) {
			let moved = match event.code {
				KeyCode::Up => Some(self.move_dialog(-1)),
				KeyCode::Down => Some(self.move_dialog(1)),
				_ => None,
			};
			if moved.is_some() {
				return moved.unwrap();
			}
		}
		match event.code {
			KeyCode::Up => self.navigate_dialog(-1),
			KeyCode::Down => self.navigate_dialog(1),
			KeyCode::Enter => self.play_dialog(false),
			KeyCode::Char('/') => self.play_dialog(true),
			KeyCode::Char('a') => self.add_dialog(),
			KeyCode::Char('e') => self.edit_dialog(),
			KeyCode::Char('r') => self.rename_dialog(),
			KeyCode::Char('d') => self.delete_dialog(),
			KeyCode::Char('f') => self.duplicate_dialog(),
			KeyCode::Char('x') => self.set_global_key_bind(),
			KeyCode::Char('z') => self.unset_global_key_bind(),
			KeyCode::Char('v') => self.set_dialog_id(),
			KeyCode::Char('b') => self.unset_dialog_id(),
			KeyCode::PageUp => self.navigate_dialog(-(self.range.1 - self.range.0 + 1)),
			KeyCode::PageDown => self.navigate_dialog(self.range.1 - self.range.0 + 1),
			KeyCode::Home => self.navigate_dialog(-i32::MAX),
			KeyCode::End => self.navigate_dialog(i32::MAX),
			_ => false
		}
	}
}

impl BlockNavigation for DialogBlock {
	const ID: u8 = 7;

	fn navigate_block(&self, dx: i16, dy: i16) -> u8 {
		if dy < 0 {
			return TabsBlock::ID;
		}
		if dx > 0 && acquire().settings_opened {
			return SettingsBlock::ID;
		}
		Self::ID
	}
}

impl DialogBlock {
	fn play_dialog(&self, random: bool) -> bool {
		let app = acquire();
		let index;
		if random {
			index = rand::thread_rng().gen_range(0..app.dialogs.len());
		} else {
			if self.selected >= app.dialogs.len() {
				return false;
			}
			index = self.selected;
		}
		app.dialogs[index].play(true);
		true
	}

	fn navigate_dialog(&mut self, dy: i32) -> bool {
		let app = acquire();
		let len = app.dialogs.len();
		let new_selected;
		if dy.abs() > 1 {
			new_selected = (self.selected as i32 + dy).clamp(0, len as i32 - 1) as usize;
		} else {
			new_selected = loop_index(self.selected, dy, len);
		}
		if new_selected != self.selected {
			self.selected = new_selected;
			return true;
		}
		false
	}

	fn move_dialog(&mut self, dy: i32) -> bool {
		let mut app = acquire();
		if self.selected == 0 && dy < 0 || self.selected == app.dialogs.len() - 1 && dy > 0 {
			return false;
		}
		app.dialogs.swap(self.selected, (self.selected as i32 + dy) as usize);
		app.config.dialogs.swap(self.selected, (self.selected as i32 + dy) as usize);
		self.selected = (self.selected as i32 + dy) as usize;
		true
	}

	fn add_dialog(&mut self) -> bool {
		let mut app = acquire();
		let dialog = Dialog::default();
		let entry = dialog.to_entry();
		app.dialogs.push(dialog);
		app.config.dialogs.push(entry);
		self.selected = app.dialogs.len() - 1;
		drop(app);
		self.edit_dialog()
	}

	fn edit_dialog(&self) -> bool {
		set_popup(PopupComponent::Dialog(DialogPopup::new(self.selected)));
		true
	}

	fn rename_dialog(&self) -> bool {
		set_popup(PopupComponent::Input(InputPopup::new(acquire().dialogs[self.selected].label.clone(), AwaitInput::DialogName)));
		true
	}

	fn delete_dialog(&self) -> bool {
		set_popup(PopupComponent::Confirm(ConfirmPopup::new(ConfirmAction::DeleteDialog)));
		true
	}
	
	fn duplicate_dialog(&mut self) -> bool {
		let mut app = acquire();
		let dialog = app.dialogs[self.selected].clone();
		let entry = dialog.to_entry();
		app.dialogs.push(dialog);
		app.config.dialogs.push(entry);
		self.selected = app.dialogs.len() - 1;
		drop(app);
		self.edit_dialog()
	}

	fn set_global_key_bind(&self) -> bool {
		let dialog = &acquire().dialogs[self.selected];
		set_popup(PopupComponent::KeyBind(KeyBindPopup::new(KeyBindFor::Dialog, dialog.keys.clone().into_iter().collect::<HashSet<Keyboard>>())));
		true
	}

	fn unset_global_key_bind(&self) -> bool {
		let mut app = acquire();
		let dialog = &mut app.dialogs[self.selected];
		dialog.keys.clear();
		app.config.dialogs[self.selected].keys.clear();
		true
	}

	fn set_dialog_id(&self) -> bool {
		let dialog = &mut acquire().dialogs[self.selected];
		let init = match dialog.id {
			Some(id) => id.to_string(),
			None => String::new(),
		};
		set_popup(PopupComponent::Input(InputPopup::new(init, AwaitInput::SetDialogId)));
		true
	}

	fn unset_dialog_id(&self) -> bool {
		let mut app = acquire();
		app.dialogs[self.selected].id = Option::None;
		app.config.dialogs[self.selected].id = Option::None;
		true
	}
}