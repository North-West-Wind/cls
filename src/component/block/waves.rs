use std::cmp::{max, min};

use crossterm::event::{KeyCode, KeyEvent};
use rand::Rng;
use ratatui::{style::{Color, Modifier, Style}, text::{Line, Span}, widgets::{Block, Borders, Padding, Paragraph}};
use substring::Substring;

use crate::{component::{block::loop_index, popup::{confirm::{ConfirmAction, ConfirmPopup}, input::{AwaitInput, InputPopup}}}, state::config_mut, util::notify_redraw};
use crate::component::popup::wave::WavePopup;
use crate::component::popup::{set_popup, PopupComponent};
use crate::state::get_mut_app;
use crate::util;
use crate::{component::block::{borders, settings::SettingsBlock, tabs::TabsBlock, BlockHandleKey, BlockNavigation, BlockRenderArea}, state::get_app, util::{global_input::keyboard_to_string, waveform::Waveform}};

pub struct WavesBlock {
	range: (i32, i32),
	pub(super) selected: usize,
}

impl Default for WavesBlock {
	fn default() -> Self {
		Self {
			range: (-1, -1),
			selected: 0,
		}
	}
}

impl BlockRenderArea for WavesBlock {
	fn render_area(&mut self, f: &mut ratatui::Frame, area: ratatui::prelude::Rect) {
		let app = get_app();
		if self.range.0 == -1 {
			self.range = (0, area.height as i32);
		}

		let (border_type, border_style) = borders(Self::ID);
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
					keys.sort();
					spans.push(Span::from(format!("{{{}}}", keys.join("+"))).style(Style::default().fg(Color::LightGreen).add_modifier(Modifier::REVERSED)));
					spans.push(Span::from(" "));
				}
				let style = if self.selected == ii {
					Style::default().fg(Color::LightBlue).add_modifier(Modifier::REVERSED)
				} else {
					Style::default().fg(Color::Cyan)
				};
				let details = if wave.waves.len() == 1 {
					format!("{:?} {:.2} Hz",  wave.waves[0].wave_type,  wave.waves[0].frequency)
				} else {
					format!("{:?} {:.2} Hz + {} more",  wave.waves[0].wave_type,  wave.waves[0].frequency, wave.waves.len() - 1)
				};
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
		match event.code {
			KeyCode::Up => self.navigate_wave(-1),
			KeyCode::Down => self.navigate_wave(1),
			KeyCode::Enter => self.play_wave(false),
			KeyCode::Char('/') => self.play_wave(true),
			KeyCode::Char('a') => self.add_wave(),
			KeyCode::Char('e') => self.edit_wave(),
			KeyCode::Char('r') => self.rename_wave(),
			KeyCode::Char('d') => self.delete_wave(),
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
		if dx > 0 && get_app().settings_opened {
			return SettingsBlock::ID;
		}
		Self::ID
	}
}

impl WavesBlock {
	fn play_wave(&self, random: bool) -> bool {
		let app = get_app();
		let index;
		if random {
			index = rand::thread_rng().gen_range(0..app.waves.len());
		} else {
			if self.selected >= app.waves.len() {
				return false;
			}
			index = self.selected;
		}
		util::waveform::play_wave(app.waves[index].clone(), true);
		true
	}

	fn navigate_wave(&mut self, dy: i32) -> bool {
		let app = get_app();
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

	fn add_wave(&mut self) -> bool {
		let app = get_mut_app();
		let config = config_mut();
		let waveform = Waveform::default();
		let entry = waveform.to_entry();
		app.waves.push(waveform);
		config.waves.push(entry);
		self.selected = app.waves.len() - 1;
		self.edit_wave()
	}

	fn edit_wave(&self) -> bool {
		set_popup(PopupComponent::Wave(WavePopup::new(self.selected)));
		true
	}

	fn rename_wave(&self) -> bool {
		set_popup(PopupComponent::Input(InputPopup::new(get_app().waves[self.selected].label.clone(), AwaitInput::WaveName)));
		true
	}

	fn delete_wave(&mut self) -> bool {
		set_popup(PopupComponent::Confirm(ConfirmPopup::new(ConfirmAction::DeleteWave)));
		true
	}
}

pub fn set_wave_name(name: String) {
	let app = get_mut_app();
	let config = config_mut();
	let selected = app.wave_selected();
	app.waves[selected].label = name.clone();
	config.waves[selected].label = name;
	notify_redraw();
}