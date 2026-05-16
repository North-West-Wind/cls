use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{layout::Rect, style::{Color, Modifier, Style}, text::Line, widgets::{Block, BorderType, Clear, Padding, Paragraph, Widget}, Frame};
use rayon::iter::{IndexedParallelIterator, IntoParallelRefIterator, ParallelIterator};
use std::thread;

use crate::{component::{popup::{PopupComponent, PopupHandleKey, PopupRender, confirm::ConfirmPopup, defer_exit_popup, defer_set_popup, input::{FLAG_NUM, InputPopup}, popups}}, state::acquire, util::wave::{Wave, WaveType, Waveform}};

pub struct WavePopup {
	index: usize,
	pub(super) waveform: Waveform,
	pub(super) selected: usize,
	pub(super) changed: bool
}

impl WavePopup {
	pub fn new(index: usize) -> Self {
		Self {
			index,
			waveform: acquire().waves[index].clone(),
			selected: 0,
			changed: false
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

		lines.push(Line::from(if self.waveform.waves.len() > page_size {
			format!("Wave List (Page {} / {})", page + 1, (self.waveform.waves.len() + page_size - 1) / page_size)
		} else {
			"Wave List".to_string()
		}).style(Style::default().add_modifier(Modifier::BOLD)).centered());

		self.waveform.waves[(page * page_size)..((page + 1) * page_size).min(self.waveform.waves.len())].par_iter().enumerate().map(|(ii, wave)| {
			Line::from(format!("{:?} {:.2} Hz x{:.2} >{:2.}", wave.wave_type, wave.frequency, wave.amplitude, wave.phase)).style(if self.selected == ii {
				Style::default().fg(Color::LightGreen).add_modifier(Modifier::REVERSED)
			} else {
				Style::default().fg(Color::Green)
			})
		}).collect::<Vec<_>>().append(&mut lines);

		let area = f.area();
		let width = lines.par_iter().map(|line| { line.width() as u16 }).sum::<u16>() + 4;
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

impl PopupHandleKey for WavePopup {
	fn handle_key(&mut self, event: KeyEvent) -> bool {
		use KeyCode::*;
		match event.code {
			Up => self.navigate_wave(-1),
			Down => self.navigate_wave(1),
			Left => self.change_type(-1),
			Right => self.change_type(1),
			Char('a') => self.add_wave(),
			Char('d') => self.delete_wave(),
			Char('f') => self.popup_frequency(),
			Char('g') => self.popup_amplitude(),
			Char('h') => self.popup_phase(),
			Enter => self.commit_changes(),
			Esc|Char('q') => self.discard_changes(),
			_ => false
		}
	}
}

impl WavePopup {
	fn navigate_wave(&mut self, dy: i16) -> bool {
		let changed = self.selected as i16 + dy;
		let new_selected: usize;
		if changed < 0 {
			new_selected = self.waveform.waves.len() - 1;
		} else if changed as usize >= self.waveform.waves.len() {
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

	fn selected_wave(&self) -> &Wave {
		&self.waveform.waves[self.selected]
	}

	fn selected_wave_mut(&mut self) -> &mut Wave {
		&mut self.waveform.waves[self.selected]
	}

	fn change_type(&mut self, dx: i16) -> bool {
		use WaveType::*;
		let wave = self.selected_wave_mut();
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
		self.waveform.waves.push(Wave::default());
		true
	}

	fn delete_wave(&mut self) -> bool {
		if self.waveform.waves.len() <= 1 {
			return false;
		}
		self.waveform.waves.remove(self.selected);
		if self.selected >= self.waveform.waves.len() {
			self.selected = self.waveform.waves.len() - 1;
		}
		true
	}

	fn popup_frequency(&self) -> bool {
		defer_set_popup(PopupComponent::Input(InputPopup::new(self.selected_wave().frequency.to_string(), "Frequency (Hz)".to_string(), FLAG_NUM, |value| {
			let Ok(freq) = value.parse::<f32>() else { return false; };
			thread::spawn(move || {
				if let Some(popup) = popups().last_mut() && let PopupComponent::Wave(popup) = popup {
					let wave = &mut popup.waveform.waves[popup.selected];
					if wave.frequency != freq {
						popup.changed = true;
					}
					wave.frequency = freq;
				}
			});
			false
		})));
		true
	}

	fn popup_amplitude(&self) -> bool {
		defer_set_popup(PopupComponent::Input(InputPopup::new(self.selected_wave().amplitude.to_string(), "Amplitude (Default = 1)".to_string(), FLAG_NUM, |value| {
			let Ok(amplitude) = value.parse::<f32>() else { return false; };
			thread::spawn(move || {
				if let Some(popup) = popups().last_mut() && let PopupComponent::Wave(popup) = popup {
					let wave = &mut popup.waveform.waves[popup.selected];
					if wave.amplitude != amplitude {
						popup.changed = true;
					}
					wave.amplitude = amplitude;
				}
			});
			false
		})));
		true
	}

	fn popup_phase(&self) -> bool {
		defer_set_popup(PopupComponent::Input(InputPopup::new(self.selected_wave().phase.to_string(), "Phase".to_string(), FLAG_NUM, |value| {
			let Ok(phase) = value.parse::<f32>() else { return false; };
			thread::spawn(move || {
				if let Some(popup) = popups().last_mut() && let PopupComponent::Wave(popup) = popup {
					let wave = &mut popup.waveform.waves[popup.selected];
					if wave.phase != phase {
						popup.changed = true;
					}
					wave.phase = phase;
				}
			});
			false
		})));
		true
	}

	fn commit_changes(&self) -> bool {
		let mut app = acquire();
		app.waves[self.index] = self.waveform.clone();
		app.config.waves[self.index] = self.waveform.to_entry();
		defer_exit_popup();
		true
	}

	fn discard_changes(&self) -> bool {
		if self.changed {
			defer_set_popup(PopupComponent::Confirm(ConfirmPopup::new("Discard changes?", "discard", || {
				defer_exit_popup();
				false
			})));
		} else {
			defer_exit_popup();
		}
		true
	}
}