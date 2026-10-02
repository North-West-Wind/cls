use std::{cmp::{max, min}, thread, time::Duration, vec};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use rand::Rng;
use ratatui::{style::{Color, Modifier, Style}, text::{Line, Span}, widgets::{Block, Borders, Padding, Paragraph}};
use rayon::iter::{IndexedParallelIterator, IntoParallelRefIterator, ParallelIterator};
use substring::Substring;

use crate::{client::{AtomicClientState, ClientState, client_ext::wave::ClientWave, component::{block::{BlockHandleKey, BlockNavigation, BlockRenderArea, loop_index, settings::SettingsBlock, tabs::TabsBlock}, popup::{PopupComponent, confirm::ConfirmPopup, input::{FLAG_NONE, InputPopup}, key_bind::KeyBindPopup, wave::WavePopup}}}, common::socket::ClientToServer};

pub struct WavesBlock {
	range: (i32, i32),
	height: u16,
}

impl Default for WavesBlock {
	fn default() -> Self {
		Self {
			range: (-1, -1),
			height: 0,
		}
	}
}

impl BlockRenderArea for WavesBlock {
	fn render_area(&mut self, client_state: &ClientState, f: &mut ratatui::Frame, area: ratatui::prelude::Rect) {
		if self.range.0 == -1 || self.height != area.height {
			self.range = (0, area.height as i32 - 5);
			self.height = area.height;
		}

		let (border_type, border_style) = client_state.borders(Self::ID);
		let block = Block::default()
			.title("Waveforms")
			.borders(Borders::ALL)
			.border_type(border_type)
			.border_style(border_style.fg(if client_state.selected_block == Self::ID { Color::LightBlue } else { Color::Blue }))
			.padding(Padding::new(2, 2, 1, 1));

		let paragraph: Paragraph;
		if client_state.waves.len() == 0 {
			paragraph = Paragraph::new("Add a waveform to get started! :>");
		} else {
			let lines = client_state.waves.par_iter().enumerate().map(|(ii, wave)| {
				let mut spans = vec![];
				if wave.base.id.is_some() {
					spans.push(Span::from("I").style(Style::default().fg(Color::LightYellow).add_modifier(Modifier::REVERSED)));
				} else {
					spans.push(Span::from(" "));
				}
				if !wave.base.keys.is_empty() {
					spans.push(Span::from("K").style(Style::default().fg(Color::LightGreen).add_modifier(Modifier::REVERSED)));
				} else {
					spans.push(Span::from(" "));
				}
				spans.push(Span::from(" "));
				let style = if client_state.selected_wave == ii {
					Style::default().fg(Color::LightBlue).add_modifier(Modifier::REVERSED)
				} else {
					Style::default().fg(Color::Cyan)
				};
				let details = wave.details();
				let label = &wave.base.label;
				let extra: usize = spans.par_iter().map(|span| { span.width() }).sum();
				if label.len() + details.len() + extra as usize > area.width as usize - 6 {
					spans.push(Span::from(label.substring(0, max(0, area.width as i32 - 10 - extra as i32 - details.len() as i32) as usize)).style(style));
					spans.push(Span::from("... ".to_owned() + &details).style(style));
				} else {
					spans.push(Span::from(label.clone()).style(style));
					spans.push(Span::from(vec![" "; max(0, area.width as i32 - 6 - extra as i32 - label.len() as i32 - details.len() as i32) as usize].join("")).style(style));
					spans.push(Span::from(details.clone()).style(style));
				}
				Line::from(spans)
			}).collect::<Vec<_>>();
			if client_state.selected_wave < self.range.0 as usize {
				self.range = (client_state.selected_wave as i32, client_state.selected_wave as i32 + area.height as i32 - 5);
			} else if client_state.selected_wave > self.range.1 as usize {
				self.range = (client_state.selected_wave as i32 - area.height as i32 + 5, client_state.selected_wave as i32);
			}
			paragraph = Paragraph::new(lines).scroll((self.range.0 as u16, 0));
		}

		f.render_widget(paragraph.block(block), area);
	}
}

impl BlockHandleKey for WavesBlock {
	fn handle_key(&mut self, client_state: AtomicClientState, event: KeyEvent) -> bool {
		if event.modifiers.contains(KeyModifiers::CONTROL) {
			let moved = match event.code {
				KeyCode::Up => Some(self.move_wave(client_state.clone(), -1)),
				KeyCode::Down => Some(self.move_wave(client_state.clone(), 1)),
				_ => None,
			};
			if moved.is_some() {
				return moved.unwrap();
			}
		}
		match event.code {
			KeyCode::Up => self.navigate_wave(client_state, -1),
			KeyCode::Down => self.navigate_wave(client_state, 1),
			KeyCode::Enter => self.play_wave(client_state, false),
			KeyCode::Char('/') => self.play_wave(client_state, true),
			KeyCode::Char('a') => self.add_wave(client_state),
			KeyCode::Char('e') => self.edit_wave(client_state),
			KeyCode::Char('r') => self.rename_wave(client_state),
			KeyCode::Char('d') => self.delete_wave(client_state),
			KeyCode::Char('f') => self.duplicate_wave(client_state),
			KeyCode::Char('x') => self.set_global_key_bind(client_state),
			KeyCode::Char('z') => self.unset_global_key_bind(client_state),
			KeyCode::Char('v') => self.set_wave_id(client_state),
			KeyCode::Char('b') => self.unset_wave_id(client_state),
			KeyCode::PageUp => self.navigate_wave(client_state, -(self.range.1 - self.range.0 + 1)),
			KeyCode::PageDown => self.navigate_wave(client_state, self.range.1 - self.range.0 + 1),
			KeyCode::Home => self.navigate_wave(client_state, -i32::MAX),
			KeyCode::End => self.navigate_wave(client_state, i32::MAX),
			_ => false
		}
	}
}

impl BlockNavigation for WavesBlock {
	const ID: u8 = 6;

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

impl WavesBlock {
	fn play_wave(&self, atomic_client_state: AtomicClientState, random: bool) -> bool {
		let client_state = atomic_client_state.read();
		let index;
		if random {
			index = rand::thread_rng().gen_range(0..client_state.waves.len());
		} else {
			if client_state.selected_wave >= client_state.waves.len() {
				return false;
			}
			index = client_state.selected_wave;
		}
		let uid = client_state.waves[index].base.uid;
		// Stop 1 second later
		let atomic_client_state = atomic_client_state.clone();
		thread::spawn(move || {
			thread::sleep(Duration::from_secs(1));
			atomic_client_state.read().request(ClientToServer::StopDialog(uid));
		});
		client_state.request(ClientToServer::PlayDialog(uid))
	}

	fn navigate_wave(&mut self, client_state: AtomicClientState, dy: i32) -> bool {
		let mut client_state = client_state.write();
		let len = client_state.waves.len();
		let new_selected = if dy.abs() > 1 {
			min(len as i32 - 1, max(0, client_state.selected_wave as i32 + dy)) as usize
		} else {
			loop_index(client_state.selected_wave, dy, len)
		};
		if new_selected != client_state.selected_wave {
			client_state.selected_wave = new_selected;
			return true;
		}
		false
	}

	fn move_wave(&mut self, client_state: AtomicClientState, dy: i32) -> bool {
		{
			let client_state = client_state.read();
			if client_state.selected_wave == 0 && dy < 0 || client_state.selected_wave == client_state.waves.len() - 1 && dy > 0 {
				return false;
			}
		}
		let mut client_state = client_state.write();
		let selected = client_state.selected_wave;
		client_state.waves.swap(selected, (selected as i32 + dy) as usize);
		client_state.selected_wave = (selected as i32 + dy) as usize;
		true
	}

	fn add_wave(&mut self, client_state: AtomicClientState) -> bool {
		{ client_state.write().waves.push(ClientWave::default()); }
		self.edit_wave(client_state)
	}

	fn edit_wave(&self, client_state: AtomicClientState) -> bool {
		let (wave, selected, popup_manager) = {
			let client_state = client_state.read();
			(client_state.waves[client_state.selected_wave].base.clone(), client_state.selected_wave, client_state.popup_manager.clone())
		};
		popup_manager.push(PopupComponent::Wave(WavePopup::new(wave, move |wave| {
			let mut client_state = client_state.write();
			client_state.waves[selected].base = wave;
		})));
		true
	}

	fn rename_wave(&self, client_state: AtomicClientState) -> bool {
		let (label, selected, popup_manager) = {
			let client_state = client_state.read();
			(client_state.waves[client_state.selected_wave].base.label.clone(), client_state.selected_wave, client_state.popup_manager.clone())
		};
		popup_manager.push(PopupComponent::Input(InputPopup::new(label, "Waveform Label".to_string(), FLAG_NONE, move |value| {
			let name = value.to_string();
			let mut client_state = client_state.write();
			client_state.waves[selected].base.label = name;
		})));
		true
	}

	fn delete_wave(&self, client_state: AtomicClientState) -> bool {
		client_state.clone().read().popup_manager.push(PopupComponent::Confirm(ConfirmPopup::new("Delete wave?", "delete", move || {
			let mut client_state = client_state.write();
			let selected = client_state.selected_wave;
			client_state.waves.remove(selected);
			let len = client_state.waves.len();
			if selected >= len && len != 0 {
				client_state.selected_wave = len - 1;
			}
		})));
		true
	}
	
	fn duplicate_wave(&mut self, client_state: AtomicClientState) -> bool {
		{
			let mut client_state = client_state.write();
			let wave = client_state.waves[client_state.selected_wave].clone();
			client_state.waves.push(wave);
			client_state.selected_wave = client_state.waves.len() - 1;
		}
		self.edit_wave(client_state)
	}

	fn set_global_key_bind(&self, client_state: AtomicClientState) -> bool {
		let (recorded, selected, popup_manager) = {
			let client_state = client_state.read();
			(client_state.waves[client_state.selected_wave].base.keys.clone().into(), client_state.selected_wave, client_state.popup_manager.clone())
		};
		popup_manager.push(PopupComponent::KeyBind(KeyBindPopup::new(recorded, move |keys| {
			let mut client_state = client_state.write();
			client_state.waves[selected].base.keys = keys;
		})));
		true
	}

	fn unset_global_key_bind(&self, client_state: AtomicClientState) -> bool {
		let mut client_state = client_state.write();
		let selected = client_state.selected_wave;
		client_state.waves[selected].base.keys.clear();
		true
	}

	fn set_wave_id(&self, client_state: AtomicClientState) -> bool {
		let (init, selected, popup_manager) = {
			let client_state = client_state.read();
			(match client_state.waves[client_state.selected_wave].base.id {
				Some(id) => id.to_string(),
				None => String::new(),
			}, client_state.selected_wave, client_state.popup_manager.clone())
		};
		popup_manager.push(PopupComponent::Input(InputPopup::new(init, "Waveform ID".to_string(), FLAG_NONE, move |value| {
			let Ok(id) = u32::from_str_radix(value, 10) else { return; };
			let mut client_state = client_state.write();
			client_state.waves[selected].base.id = Some(id);
		})));
		true
	}

	fn unset_wave_id(&self, client_state: AtomicClientState) -> bool {
		let mut client_state = client_state.write();
		let selected = client_state.selected_wave;
		client_state.waves[selected].base.id = None;
		true
	}
}