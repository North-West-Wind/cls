
use std::{thread, time::{Duration, SystemTime}, vec};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use rand::Rng;
use ratatui::{Frame, layout::Rect, style::{Color, Modifier, Style}, text::{Line, Span}, widgets::{Block, Borders, Padding, Paragraph}};
use rayon::iter::{IndexedParallelIterator, IntoParallelRefIterator, ParallelIterator};

use crate::{client::{AtomicClientState, ClientState, component::{block::{BlockHandleKey, BlockNavigation, BlockRenderArea, loop_index, settings::SettingsBlock, tabs::TabsBlock}, popup::{PopupComponent, confirm::ConfirmPopup, dialog::DialogPopup, input::{FLAG_NONE, InputPopup}, key_bind::KeyBindPopup}}}, common::{base::dialog::Dialog, socket::ClientToServer}};

pub struct DialogBlock {
	range: (i32, i32),
	height: u16,
}

impl Default for DialogBlock {
	fn default() -> Self {
		Self {
			range: (-1, -1),
			height: 0,
		}
	}
}

impl BlockRenderArea for DialogBlock {
	fn render_area(&mut self, client_state: &ClientState, f: &mut Frame, area: Rect) {
		if self.range.0 == -1 || self.height != area.height {
			self.range = (0, area.height as i32 - 5);
			self.height = area.height;
		}

		let (border_type, border_style) = client_state.borders(Self::ID);
		let block = Block::default()
			.title("Dialog")
			.borders(Borders::ALL)
			.border_type(border_type)
			.border_style(border_style.fg(Color::Yellow))
			.padding(Padding::new(2, 2, 1, 1));

		let paragraph: Paragraph;
		if client_state.dialogs.len() == 0 {
			paragraph = Paragraph::new("Add a dialog to get started! :>");
		} else {
			let lines = client_state.dialogs.par_iter().enumerate().map(|(ii, dialog)| {
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
				let style = if client_state.selected_dialog == ii {
					Style::default().fg(Color::LightYellow).add_modifier(Modifier::REVERSED)
				} else {
					Style::default().fg(Color::Yellow)
				};
				spans.push(Span::from(dialog.label.clone()).style(style));
				Line::from(spans)
			}).collect::<Vec<_>>();
			if client_state.selected_dialog < self.range.0 as usize {
				self.range = (client_state.selected_dialog as i32, client_state.selected_dialog as i32 + area.height as i32 - 5);
			} else if client_state.selected_dialog > self.range.1 as usize {
				self.range = (client_state.selected_dialog as i32 - area.height as i32 + 5, client_state.selected_dialog as i32);
			}
			paragraph = Paragraph::new(lines).scroll((self.range.0 as u16, 0));
		}

		f.render_widget(paragraph.block(block), area);
	}
}

impl BlockHandleKey for DialogBlock {
	fn handle_key(&mut self, client_state: AtomicClientState, event: KeyEvent) -> bool {
		if event.modifiers.contains(KeyModifiers::CONTROL) {
			let moved = match event.code {
				KeyCode::Up => Some(self.move_dialog(&mut client_state.write(), -1)),
				KeyCode::Down => Some(self.move_dialog(&mut client_state.write(), 1)),
				_ => None,
			};
			if moved.is_some() {
				return moved.unwrap();
			}
		}
		match event.code {
			KeyCode::Up => self.navigate_dialog(&mut client_state.write(), -1),
			KeyCode::Down => self.navigate_dialog(&mut client_state.write(), 1),
			KeyCode::Enter => self.play_dialog(client_state, false),
			KeyCode::Char('/') => self.play_dialog(client_state, true),
			KeyCode::Char('a') => self.add_dialog(client_state),
			KeyCode::Char('e') => self.edit_dialog(client_state),
			KeyCode::Char('r') => self.rename_dialog(client_state),
			KeyCode::Char('d') => self.delete_dialog(client_state),
			KeyCode::Char('f') => self.duplicate_dialog(client_state),
			KeyCode::Char('x') => self.set_global_key_bind(client_state),
			KeyCode::Char('z') => self.unset_global_key_bind(&mut client_state.write()),
			KeyCode::Char('v') => self.set_dialog_id(client_state),
			KeyCode::Char('b') => self.unset_dialog_id(&mut client_state.write()),
			KeyCode::PageUp => self.navigate_dialog(&mut client_state.write(), -(self.range.1 - self.range.0 + 1)),
			KeyCode::PageDown => self.navigate_dialog(&mut client_state.write(), self.range.1 - self.range.0 + 1),
			KeyCode::Home => self.navigate_dialog(&mut client_state.write(), -i32::MAX),
			KeyCode::End => self.navigate_dialog(&mut client_state.write(), i32::MAX),
			_ => false
		}
	}
}

impl BlockNavigation for DialogBlock {
	const ID: u8 = 7;

	fn navigate_block(&self, client_state: AtomicClientState, dx: i16, dy: i16) -> u8 {
		if dy < 0 {
			return TabsBlock::ID;
		}
		if dx > 0 && client_state.read().settings_opened {
			return SettingsBlock::ID;
		}
		Self::ID
	}
}

impl DialogBlock {
	fn play_dialog(&self, atomic_client_state: AtomicClientState, random: bool) -> bool {
		let (selected, length) = {
			let client_state = atomic_client_state.read();
			(client_state.selected_dialog, client_state.dialogs.len())
		};
		let index;
		if random {
			index = rand::thread_rng().gen_range(0..length);
		} else {
			if selected >= length {
				return false;
			}
			index = selected;
		}
		let client_state = atomic_client_state.read();
		let uid = client_state.dialogs[index].uid;
		// Stop 1 second later
		let atomic_client_state = atomic_client_state.clone();
		thread::spawn(move || {
			thread::sleep(Duration::from_secs(1));
			atomic_client_state.read().request(ClientToServer::StopDialog(uid));
		});
		client_state.request(ClientToServer::PlayDialog(uid))
	}

	fn navigate_dialog(&mut self, client_state: &mut ClientState, dy: i32) -> bool {
		let (selected, length) = (client_state.selected_dialog, client_state.dialogs.len());
		let new_selected = if dy.abs() > 1 {
			(selected as i32 + dy).clamp(0, length as i32 - 1) as usize
		} else {
			loop_index(selected, dy, length)
		};
		if new_selected != selected {
			client_state.selected_dialog = new_selected;
			return true;
		}
		false
	}

	fn move_dialog(&mut self, client_state: &mut ClientState, dy: i32) -> bool {
		let (selected, length) = (client_state.selected_dialog, client_state.dialogs.len());
		if selected == 0 && dy < 0 || selected == length - 1 && dy > 0 {
			return false;
		}
		client_state.dialogs.swap(selected, (selected as i32 + dy) as usize);
		client_state.selected_dialog = (selected as i32 + dy) as usize;
		true
	}

	fn add_dialog(&mut self, client_state: AtomicClientState) -> bool {
		{
			let mut client_state = client_state.write();
			let dialog = Dialog::default();
			client_state.request(ClientToServer::SetDialog(dialog.uid, dialog.to_saveable()));
			client_state.dialogs.push(dialog);
			client_state.selected_dialog = client_state.dialogs.len() - 1;
		}
		self.edit_dialog(client_state)
	}

	fn edit_dialog(&self, atomic_client_state: AtomicClientState) -> bool {
		let (dialog, selected, popups) = {
			let client_state = atomic_client_state.read();
			(client_state.dialogs[client_state.selected_dialog].clone(), client_state.selected_dialog, client_state.popup_manager.clone())
		};
		popups.push(PopupComponent::Dialog(DialogPopup::new(dialog, move |dialog| {
			let mut client_state = atomic_client_state.write();
			client_state.request(ClientToServer::SetDialog(dialog.uid, dialog.to_saveable()));
			client_state.dialogs[selected] = dialog;
		})));
		true
	}

	fn rename_dialog(&self, atomic_client_state: AtomicClientState) -> bool {
		let (label, selected, popups) = {
			let client_state = atomic_client_state.read();
			(client_state.dialogs[client_state.selected_dialog].label.clone(), client_state.selected_dialog, client_state.popup_manager.clone())
		};
		popups.push(PopupComponent::Input(InputPopup::new(label, "Dialog Label".to_string(), FLAG_NONE, move |value| {
			let name: String = value.to_string();
			let mut client_state = atomic_client_state.write();
			client_state.dialogs[selected].label = name.clone();
			let dialog = &client_state.dialogs[selected];
			client_state.request(ClientToServer::SetDialog(dialog.uid, dialog.to_saveable()));
		})));
		true
	}

	fn delete_dialog(&self, client_state: AtomicClientState) -> bool {
		let popups = client_state.read().popup_manager.clone();
		popups.push(PopupComponent::Confirm(ConfirmPopup::new("Delete dialog?", "delete", move || {
			let mut client_state = client_state.write();
			let selected = client_state.selected_dialog;
			let dialog = client_state.dialogs.remove(selected);
			let len = client_state.dialogs.len();
			if selected >= len && len != 0 {
				client_state.selected_dialog = len - 1;
			}
			client_state.request(ClientToServer::DeleteDialog(dialog.uid));
		})));
		true
	}
	
	fn duplicate_dialog(&mut self, client_state: AtomicClientState) -> bool {
		{
			let mut client_state = client_state.write();
			let mut dialog = client_state.dialogs[client_state.selected_dialog].clone();
			dialog.uid = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_millis() as u64;
			client_state.request(ClientToServer::SetDialog(dialog.uid, dialog.to_saveable()));
			client_state.dialogs.push(dialog);
			client_state.selected_dialog = client_state.dialogs.len() - 1;
		}
		self.edit_dialog(client_state)
	}

	fn set_global_key_bind(&self, atomic_client_state: AtomicClientState) -> bool {
		let (keys, selected, popups) = {
			let client_state = atomic_client_state.read();
			(client_state.dialogs[client_state.selected_dialog].keys.clone(), client_state.selected_dialog, client_state.popup_manager.clone())
		};
		popups.push(PopupComponent::KeyBind(KeyBindPopup::new(keys, move |keys| {
			let mut client_state = atomic_client_state.write();
			client_state.dialogs[selected].keys = keys;
			let dialog = &client_state.dialogs[selected];
			client_state.request(ClientToServer::SetDialog(dialog.uid, dialog.to_saveable()));
		})));
		true
	}

	fn unset_global_key_bind(&self, client_state: &mut ClientState) -> bool {
		let selected = client_state.selected_dialog;
		client_state.dialogs[selected].keys.clear();
		let dialog = &client_state.dialogs[selected];
		client_state.request(ClientToServer::SetDialog(dialog.uid, dialog.to_saveable()));
		true
	}

	fn set_dialog_id(&self, atomic_client_state: AtomicClientState) -> bool {
		let (init, selected, popups) = {
			let client_state = atomic_client_state.read();
			(
				match client_state.dialogs[client_state.selected_dialog].id {
					Some(id) => id.to_string(),
					None => String::new(),
				}, client_state.selected_dialog, client_state.popup_manager.clone()
			)
		};
		popups.push(PopupComponent::Input(InputPopup::new(init, "Dialog ID".to_string(), FLAG_NONE, move |value| {
			let Ok(id) = u32::from_str_radix(&value, 10) else { return; };
			let mut client_state = atomic_client_state.write();
			client_state.dialogs[selected].id = Some(id);
			let dialog = &client_state.dialogs[selected];
			client_state.request(ClientToServer::SetDialog(dialog.uid, dialog.to_saveable()));
		})));
		true
	}

	fn unset_dialog_id(&self, client_state: &mut ClientState) -> bool {
		let selected = client_state.selected_dialog;
		client_state.dialogs[selected].id = None;
		let dialog = &client_state.dialogs[selected];
		client_state.request(ClientToServer::SetDialog(dialog.uid, dialog.to_saveable()));
		true
	}
}