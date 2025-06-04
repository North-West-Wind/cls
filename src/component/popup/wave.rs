use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{layout::Rect, style::{Color, Modifier, Style}, text::Line, widgets::{Block, BorderType, Clear, Padding, Paragraph, Widget}, Frame};
use std::cmp::max;

use crate::{component::popup::{confirm::{ConfirmAction, ConfirmPopup}, exit_popup, input::{AwaitInput, InputPopup}, set_popup, PopupComponent, PopupHandleKey, PopupRender}, state::{config_mut, get_app, get_mut_app}, util::waveform::{Wave, WaveType, Waveform}};

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
			waveform: get_app().waves[index].clone(),
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

			Line::from("Wave List")
		];

		for (ii, wave) in self.waveform.waves.iter().enumerate() {
			lines.push(Line::from(format!("{:?} {:.2} Hz x{:.2} >{:2.}", wave.wave_type, wave.frequency, wave.amplitude, wave.phase))
				.style(if self.selected == ii {
					Style::default().fg(Color::LightGreen).add_modifier(Modifier::REVERSED)
				} else {
					Style::default().fg(Color::Green)
				}));
		}

		let area = f.area();
		let width = lines.iter()
			.map(|line| { line.width() })
			.fold(0, |acc, width| max(acc, width)) as u16 + 4;
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
			Esc => self.discard_changes(),
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
			self.selected = self.waveform.waves.len();
		}
		true
	}

	fn popup_frequency(&self) -> bool {
		set_popup(PopupComponent::Input(InputPopup::new(self.selected_wave().frequency.to_string(), AwaitInput::WaveFrequency)));
		true
	}

	fn popup_amplitude(&self) -> bool {
		set_popup(PopupComponent::Input(InputPopup::new(self.selected_wave().amplitude.to_string(), AwaitInput::WaveAmplitude)));
		true
	}

	fn popup_phase(&self) -> bool {
		set_popup(PopupComponent::Input(InputPopup::new(self.selected_wave().phase.to_string(), AwaitInput::WavePhase)));
		true
	}

	fn commit_changes(&self) -> bool {
		get_mut_app().waves[self.index] = self.waveform.clone();
		config_mut().waves[self.index] = self.waveform.to_entry();
		exit_popup();
		true
	}

	fn discard_changes(&self) -> bool {
		if self.changed {
			set_popup(PopupComponent::Confirm(ConfirmPopup::new(ConfirmAction::DiscardWaveChanges)));
		} else {
			exit_popup();
		}
		true
	}
}

pub(super) fn get_wave_popup() -> Option<&'static mut WavePopup> {
	return get_mut_app().popups.iter_mut().find_map(|popup| {
		match popup {
			PopupComponent::Wave(popup) => { Option::Some(popup) },
			_ => Option::None
		}
	});
}