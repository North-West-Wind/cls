use std::{cmp::{max, min}, collections::HashSet, sync::{Arc, LazyLock, Mutex, MutexGuard}};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use mki::Keyboard;
use rand::Rng;
use ratatui::{style::{Color, Modifier, Style}, text::{Line, Span}, widgets::{Block, Borders, Padding, Paragraph}};
use substring::Substring;

use crate::{component::{block::{loop_index, BlockSingleton}, popup::{confirm::{ConfirmAction, ConfirmPopup}, input::{AwaitInput, InputPopup}, key_bind::{KeyBindFor, KeyBindPopup}}}, state::notify_redraw, util::{global_input::sort_keys, waveform::play_wave}};
use crate::component::popup::wave::WavePopup;
use crate::component::popup::{set_popup, PopupComponent};
use crate::{component::block::{settings::SettingsBlock, tabs::TabsBlock, BlockHandleKey, BlockNavigation, BlockRenderArea}, state::acquire, util::{global_input::keyboard_to_string, waveform::Waveform}};

pub struct WavesBlock {
	range: (i32, i32),
	height: u16,
	pub selected: usize,
}

impl BlockSingleton for WavesBlock {
	fn instance() -> MutexGuard<'static, Self> {
		static BLOCK: LazyLock<Mutex<WavesBlock>> = LazyLock::new(|| { Mutex::new(WavesBlock {
			range: (-1, -1),
			height: 0,
			selected: 0
		}) });
		BLOCK.lock().unwrap()
	}
}

impl BlockRenderArea for WavesBlock {
	fn render_area(&mut self, f: &mut ratatui::Frame, area: ratatui::prelude::Rect) {
		if self.range.0 == -1 || self.height != area.height {
			self.range = (0, area.height as i32 - 5);
			self.height = area.height;
		}

		let app = acquire();
		let (border_type, border_style) = app.borders(Self::ID);
		let block = Block::default()
			.title("Waveforms")
			.borders(Borders::ALL)
			.border_type(border_type)
			.border_style(border_style.fg(if app.block_selected == Self::ID { Color::LightBlue } else { Color::Blue }))
			.padding(Padding::new(2, 2, 1, 1));

		let paragraph: Paragraph;
		if app.waves.len() == 0 {
			paragraph = Paragraph::new("Add a waveform to get started! :>");
		} else {
			let mut lines = vec![];
			for (ii, wave) in app.waves.iter().enumerate() {
				let mut spans = vec![];
				wave.id.inspect(|id| {
					spans.push(Span::from(format!("({})", id)).style(Style::default().fg(Color::LightYellow).add_modifier(Modifier::REVERSED)));
					spans.push(Span::from(" "));
				});
				if !wave.keys.is_empty() {
					let mut keys = wave.keys.iter()
						.map(|key| { keyboard_to_string(*key) })
						.collect::<Vec<String>>();
					let keys = sort_keys(&mut keys);
					spans.push(Span::from(format!("{{{}}}", keys.join(" "))).style(Style::default().fg(Color::LightGreen).add_modifier(Modifier::REVERSED)));
					spans.push(Span::from(" "));
				}
				let style = if self.selected == ii {
					Style::default().fg(Color::LightBlue).add_modifier(Modifier::REVERSED)
				} else {
					Style::default().fg(Color::Cyan)
				};
				let details = wave.details();
				let label = &wave.label;
				let extra = spans.iter()
					.map(|span| { span.width() as i32 })
					.fold(0, |acc, width| { acc + width });
				if label.len() + details.len() + extra as usize > area.width as usize - 6 {
					spans.push(Span::from(label.substring(0, max(0, area.width as i32 - 10 - extra - details.len() as i32) as usize)).style(style));
					spans.push(Span::from("... ".to_owned() + &details).style(style));
				} else {
					spans.push(Span::from(label.clone()).style(style));
					spans.push(Span::from(vec![" "; max(0, area.width as i32 - 6 - extra - label.len() as i32 - details.len() as i32) as usize].join("")).style(style));
					spans.push(Span::from(details.clone()).style(style));
				}
				lines.push(Line::from(spans));
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

impl BlockHandleKey for WavesBlock {
	fn handle_key(&mut self, event: KeyEvent) -> bool {
		if event.modifiers.contains(KeyModifiers::CONTROL) {
			let moved = match event.code {
				KeyCode::Up => Some(self.move_wave(-1)),
				KeyCode::Down => Some(self.move_wave(1)),
				_ => None,
			};
			if moved.is_some() {
				return moved.unwrap();
			}
		}
		match event.code {
			KeyCode::Up => self.navigate_wave(-1),
			KeyCode::Down => self.navigate_wave(1),
			KeyCode::Enter => self.play_wave(false),
			KeyCode::Char('/') => self.play_wave(true),
			KeyCode::Char('a') => self.add_wave(),
			KeyCode::Char('e') => self.edit_wave(),
			KeyCode::Char('r') => self.rename_wave(),
			KeyCode::Char('d') => self.delete_wave(),
			KeyCode::Char('f') => self.duplicate_wave(),
			KeyCode::Char('x') => self.set_global_key_bind(),
			KeyCode::Char('z') => self.unset_global_key_bind(),
			KeyCode::Char('v') => self.set_wave_id(),
			KeyCode::Char('b') => self.unset_wave_id(),
			KeyCode::PageUp => self.navigate_wave(-(self.range.1 - self.range.0 + 1)),
			KeyCode::PageDown => self.navigate_wave(self.range.1 - self.range.0 + 1),
			KeyCode::Home => self.navigate_wave(-i32::MAX),
			KeyCode::End => self.navigate_wave(i32::MAX),
			_ => false
		}
	}
}

impl BlockNavigation for WavesBlock {
	const ID: u8 = 6;

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

impl WavesBlock {
	fn play_wave(&self, random: bool) -> bool {
		let app = acquire();
		let index;
		if random {
			index = rand::thread_rng().gen_range(0..app.waves.len());
		} else {
			if self.selected >= app.waves.len() {
				return false;
			}
			index = self.selected;
		}
		play_wave(app.waves[index].clone(), true);
		true
	}

	fn navigate_wave(&mut self, dy: i32) -> bool {
		let app = acquire();
		let len = app.waves.len();
		let new_selected;
		if dy.abs() > 1 {
			new_selected = min(len as i32 - 1, max(0, self.selected as i32 + dy)) as usize;
		} else {
			new_selected = loop_index(self.selected, dy, len);
		}
		if new_selected != self.selected {
			self.selected = new_selected;
			return true;
		}
		false
	}

	fn move_wave(&mut self, dy: i32) -> bool {
		let mut app = acquire();
		if self.selected == 0 && dy < 0 || self.selected == app.waves.len() - 1 && dy > 0 {
			return false;
		}
		app.waves.swap(self.selected, (self.selected as i32 + dy) as usize);
		app.config.waves.swap(self.selected, (self.selected as i32 + dy) as usize);
		self.selected = (self.selected as i32 + dy) as usize;
		true
	}

	fn add_wave(&mut self) -> bool {
		let mut app = acquire();
		let waveform = Waveform::default();
		let entry = waveform.to_entry();
		app.waves.push(waveform);
		app.config.waves.push(entry);
		self.selected = app.waves.len() - 1;
		drop(app);
		self.edit_wave()
	}

	fn edit_wave(&self) -> bool {
		set_popup(PopupComponent::Wave(WavePopup::new(self.selected)));
		true
	}

	fn rename_wave(&self) -> bool {
		set_popup(PopupComponent::Input(InputPopup::new(acquire().waves[self.selected].label.clone(), AwaitInput::WaveName)));
		true
	}

	fn delete_wave(&self) -> bool {
		set_popup(PopupComponent::Confirm(ConfirmPopup::new(ConfirmAction::DeleteWave)));
		true
	}
	
	fn duplicate_wave(&mut self) -> bool {
		let mut app = acquire();
		let mut waveform = app.waves[self.selected].clone();
		waveform.playing = Arc::new(Mutex::new((false, false)));
		let entry = waveform.to_entry();
		app.waves.push(waveform);
		app.config.waves.push(entry);
		self.selected = app.waves.len() - 1;
		drop(app);
		self.edit_wave()
	}

	fn set_global_key_bind(&self) -> bool {
		let wave = &acquire().waves[self.selected];
		set_popup(PopupComponent::KeyBind(KeyBindPopup::new(KeyBindFor::Wave, wave.keys.clone().into_iter().collect::<HashSet<Keyboard>>())));
		true
	}

	fn unset_global_key_bind(&self) -> bool {
		let mut app = acquire();
		let wave = &mut app.waves[self.selected];
		wave.keys.clear();
		app.config.waves[self.selected].keys.clear();
		true
	}

	fn set_wave_id(&self) -> bool {
		let wave = &mut acquire().waves[self.selected];
		let init = match wave.id {
			Some(id) => id.to_string(),
			None => String::new(),
		};
		set_popup(PopupComponent::Input(InputPopup::new(init, AwaitInput::SetWaveId)));
		true
	}

	fn unset_wave_id(&self) -> bool {
		let mut app = acquire();
		app.waves[self.selected].id = Option::None;
		app.config.waves[self.selected].id = Option::None;
		true
	}
}

pub fn set_wave_name(name: String) {
	let mut app = acquire();
	let selected = { WavesBlock::instance().selected };
	app.waves[selected].label = name.clone();
	app.config.waves[selected].label = name;
	notify_redraw();
}