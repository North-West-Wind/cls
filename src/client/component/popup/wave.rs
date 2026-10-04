use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{layout::Rect, style::{Color, Modifier, Style}, text::Line, widgets::{Block, BorderType, Clear, Padding, Paragraph, Widget}, Frame};
use rayon::iter::{IndexedParallelIterator, IntoParallelRefIterator, ParallelIterator};
use std::{format, sync::Arc, thread, vec};

use crate::{client::{ClientState, component::popup::{PopupComponent, PopupHandleKey, PopupRender, confirm::ConfirmPopup, input::{FLAG_NUM, InputPopup}}}, common::base::wave::{SingleWave, Wave, WaveType}};

pub struct WavePopup {
	pub(super) wave: Wave,
	pub(super) selected: usize,
	pub(super) changed: bool,
	pub(super) on_commit: Arc<Box<dyn Fn(Wave) + Send + Sync>>
}

impl WavePopup {
	pub fn new(wave: Wave, on_commit: impl Fn(Wave) + Send + Sync + 'static) -> Self {
		Self {
			wave: wave.clone(),
			selected: 0,
			changed: false,
			on_commit: Arc::new(Box::new(on_commit))
		}
	}
}

impl PopupRender for WavePopup {
	fn render(&self, f: &mut Frame) {
		let mut lines = vec![
			Line::from("Controls").style(Style::default().add_modifier(Modifier::BOLD)).centered(),
			Line::from("a - add, d - delete"),
			Line::from("up / down - select"),
			Line::from("left / right - change type"),
			Line::from("f / g / h - change frequency / amplitude / phase"),
			Line::from("enter / esc - save / discard changes"),
			Line::from(""),
		];

		let page_size = f.area().height as usize - 3 - lines.len();
		let page = self.selected / page_size;

		lines.push(Line::from(if self.wave.waves.len() > page_size {
			format!("Wave List (Page {} / {})", page + 1, (self.wave.waves.len() + page_size - 1) / page_size)
		} else {
			"Wave List".to_string()
		}).style(Style::default().add_modifier(Modifier::BOLD)).centered());

		lines.extend(self.wave.waves[(page * page_size)..((page + 1) * page_size).min(self.wave.waves.len())].par_iter().enumerate().map(|(ii, wave)| {
			Line::from(format!("{:?} {:.2} Hz x{:.2} >{:2.}", wave.wave_type, wave.frequency, wave.amplitude, wave.phase)).style(if self.selected == ii {
				Style::default().fg(Color::LightGreen).add_modifier(Modifier::REVERSED)
			} else {
				Style::default().fg(Color::Green)
			})
		}).collect::<Vec<_>>());

		let area = f.area();
		let width = lines.par_iter().map(|line| { line.width() as u16 }).max().unwrap() + 4;
		let height = lines.len() as u16 + 2;

		let block = Block::bordered()
			.padding(Padding::horizontal(1))
			.border_type(BorderType::Rounded)
			.title("Editor");

		if width > area.width - 4 || height > area.height {
			let popup_area = Rect {
				x: (area.width - 25) / 2,
				y: (area.height - 4) / 2,
				width: 25,
				height: 4
			};
			f.render_widget(Paragraph::new(vec![
				Line::from("Window size too small"),
				Line::from(format!("Need at least {}x{}", width, height))
			]).block(block).style(Style::default().fg(Color::Red)), popup_area);
			return;
		}

		let popup_area = Rect {
			x: (area.width - width) / 2,
			y: (area.height - height) / 2,
			width,
			height
		};

		Clear.render(popup_area, f.buffer_mut());
		f.render_widget(Paragraph::new(lines).block(block), popup_area);
	}
}

impl PopupHandleKey for WavePopup {
	fn handle_key(&mut self, client_state: &ClientState, event: KeyEvent) -> bool {
		use KeyCode::*;
		match event.code {
			Up => self.navigate_wave(-1),
			Down => self.navigate_wave(1),
			Left => self.change_type(-1),
			Right => self.change_type(1),
			Char('a') => self.add_wave(),
			Char('d') => self.delete_wave(),
			Char('f') => self.popup_frequency(client_state),
			Char('g') => self.popup_amplitude(client_state),
			Char('h') => self.popup_phase(client_state),
			Enter => self.commit_changes(client_state),
			Esc|Char('q') => self.discard_changes(client_state),
			_ => false
		}
	}
}

impl WavePopup {
	fn navigate_wave(&mut self, dy: i16) -> bool {
		let changed = self.selected as i16 + dy;
		let new_selected: usize;
		if changed < 0 {
			new_selected = self.wave.waves.len() - 1;
		} else if changed as usize >= self.wave.waves.len() {
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

	fn change_type(&mut self, dx: i16) -> bool {
		use WaveType::*;
		let wave = &mut self.wave.waves[self.selected];
		wave.wave_type = if dx > 0 {
			match wave.wave_type {
				Sine => Square,
				Square => Triangle, 
				Triangle => Saw,
				Saw => Sine
			}
		} else {
			match wave.wave_type {
				Sine => Saw,
				Square => Sine,
				Triangle => Square,
				Saw => Triangle
			}
		};
		true
	}

	fn add_wave(&mut self) -> bool {
		self.wave.waves.push(SingleWave::default());
		true
	}

	fn delete_wave(&mut self) -> bool {
		if self.wave.waves.len() <= 1 {
			return false;
		}
		self.wave.waves.remove(self.selected);
		if self.selected >= self.wave.waves.len() {
			self.selected = self.wave.waves.len() - 1;
		}
		true
	}

	fn popup_frequency(&self, client_state: &ClientState) -> bool {
		let popup_manager = client_state.popup_manager.clone();
		client_state.popup_manager.push(PopupComponent::Input(InputPopup::new(self.wave.waves[self.selected].frequency.to_string(), "Frequency (Hz)".to_string(), FLAG_NUM, move |value| {
			let Ok(freq) = value.parse::<f32>() else { return; };
			let mut popups = popup_manager.popups.lock();
			if let Some(PopupComponent::Wave(popup)) = popups.last_mut() {
				let wave = &mut popup.wave.waves[popup.selected];
				if wave.frequency != freq {
					popup.changed = true;
					wave.frequency = freq;
				}
			}
		})));
		true
	}

	fn popup_amplitude(&self, client_state: &ClientState) -> bool {
		let popup_manager = client_state.popup_manager.clone();
		client_state.popup_manager.push(PopupComponent::Input(InputPopup::new(self.wave.waves[self.selected].amplitude.to_string(), "Amplitude (Default = 1)".to_string(), FLAG_NUM, move |value| {
			let Ok(amplitude) = value.parse::<f32>() else { return; };
			let mut popups = popup_manager.popups.lock();
			if let Some(PopupComponent::Wave(popup)) = popups.last_mut() {
				let wave = &mut popup.wave.waves[popup.selected];
				if wave.amplitude != amplitude {
					popup.changed = true;
					wave.amplitude = amplitude;
				}
			}
		})));
		true
	}

	fn popup_phase(&self, client_state: &ClientState) -> bool {
		let popup_manager = client_state.popup_manager.clone();
		client_state.popup_manager.push(PopupComponent::Input(InputPopup::new(self.wave.waves[self.selected].phase.to_string(), "Amplitude (Default = 1)".to_string(), FLAG_NUM, move |value| {
			let Ok(phase) = value.parse::<f32>() else { return; };
			let mut popups = popup_manager.popups.lock();
			if let Some(PopupComponent::Wave(popup)) = popups.last_mut() {
				let wave = &mut popup.wave.waves[popup.selected];
				if wave.phase != phase {
					popup.changed = true;
					wave.phase = phase;
				}
			}
		})));
		true
	}

	fn commit_changes(&self, client_state: &ClientState) -> bool {
		let callback = self.on_commit.clone();
		let wave = self.wave.clone();
		let popups = client_state.popup_manager.clone();
		thread::spawn(move || {
			popups.pop();
			(callback)(wave);
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