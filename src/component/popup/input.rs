use std::{path::Path, thread};

use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use normpath::PathExt;
use ratatui::{style::{Color, Style}, widgets::{Block, BorderType, Clear, Padding, Paragraph, Widget}, Frame};
use tui_input::{Input, InputRequest, backend::crossterm::EventHandler};

use crate::{component::{block::{tabs::TabsBlock, waves::{set_wave_name, WavesBlock}, BlockSingleton}, popup::{defer_exit_popup, popups, PopupComponent}}, config::FileEntry, state::{acquire, Scanning}, util::{pulseaudio::{loopback, unload_module}, selected_file_path, threads::spawn_scan_thread}};

use super::{safe_centered_rect, PopupHandleKey, PopupHandlePaste, PopupRender};

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum AwaitInput {
	None,
	AddTab,
	Loopback1,
	Loopback2,
	SetFileId,
	SetWaveId,
	WaveFrequency,
	WaveAmplitude,
	WavePhase,
	WaveName,
}

pub struct InputPopup {
	input: Input,
	await_input: AwaitInput,
}

impl Default for InputPopup {
	fn default() -> Self {
		Self {
			input: Input::default(),
			await_input: AwaitInput::None,
		}
	}
}

impl InputPopup {
	pub fn new(value: String, await_input: AwaitInput) -> Self {
		Self {
			input: Input::new(value),
			await_input,
		}
	}
}

impl PopupRender for InputPopup {
	fn render(&self, f: &mut Frame) {
		use AwaitInput::*;

		let area = f.area();
		let width = (area.width / 2).max(5);
		let height = 3;
		let input = &self.input;
		let scroll = input.visual_scroll(width as usize - 5);
		let input_para = Paragraph::new(input.value())
			.scroll((0, scroll as u16))
			.block(Block::bordered().border_type(BorderType::Rounded).border_style(Style::default().fg(Color::Green)).title(match self.await_input {
				AddTab => "Add directory as tab",
				Loopback1 => "Loopback 1",
				Loopback2 => "Loopback 2",
				SetFileId => "File ID",
				SetWaveId => "Wave ID",
				WaveFrequency => "Frequency (Hz)",
				WaveAmplitude => "Amplitude (default = 1)",
				WavePhase => "Phase",
				WaveName => "Waveform Label",
				_ => "Input"
			}).padding(Padding::horizontal(1)).style(Style::default().fg(Color::Cyan)));
		let input_area = safe_centered_rect(width, height, area);
		Clear.render(input_area, f.buffer_mut());
		f.render_widget(input_para, input_area);
		f.set_cursor_position((
			input_area.x + ((input.visual_cursor()).max(scroll) - scroll) as u16 + 2,
			input_area.y + 1
		));
	}
}

impl PopupHandleKey for InputPopup {
	fn handle_key(&mut self, event: KeyEvent) -> bool {
		use AwaitInput::*;
		match event.code {
			KeyCode::Enter => self.complete(true),
			KeyCode::Esc => self.complete(false),
			KeyCode::Char(c) => {
				if c == 'h' && event.modifiers == KeyModifiers::CONTROL {
					// ctrl+backspace is parsed as ctrl+h
					self.input.handle(InputRequest::DeletePrevWord);
					true
				} else if match self.await_input {
					SetFileId|SetWaveId => c.is_digit(10),
					_ => true
				} {
					self.input.handle_event(&Event::Key(event));
					true
				} else {
					false
				}
			},
			KeyCode::Delete => {
				if event.modifiers == KeyModifiers::CONTROL {
					self.input.handle(InputRequest::DeleteNextWord);
				} else {
					self.input.handle_event(&Event::Key(event));
				}
				true
			},
			KeyCode::Tab => {
				if self.await_input == AwaitInput::AddTab {
					// Try to auto-fill path
					let input = self.input.value();
					let parent: &Path;
					let file_name: String;
					let path = Path::new(input);
					if input.ends_with("/") {
						parent = path;
						file_name = String::new();
					} else {
						let Some(path_parent) = path.parent() else { return false };
						parent = path_parent;
						let path_file_name = path.file_name();
						if let Some(os_str) = path_file_name {
							if let Some(str) = os_str.to_str() {
								file_name = str.to_string();
							} else {
								file_name = String::new();
							}
						} else {
							file_name = String::new();
						}
					}
					let Ok(files) = parent.read_dir() else { return false };
					let mut file_names = vec![];
					for file in files {
						let Ok(file) = file else { continue };
						let Ok(file_type) = file.file_type() else { continue };
						if !file_type.is_dir() { continue; }
						let os_str = file.file_name();
						let Some(file) = os_str.to_str() else { continue };
						file_names.push(file.to_string());
					}
					if file_name.is_empty() {
						if file_names.len() == 1 {
							let joined = parent.join(file_names[0].clone());
							self.input = self.input.clone().with_value(joined.to_str().unwrap().to_string());
						} else {
							return false;
						}
					} else {
						let mut prefix = String::new();
						for file in file_names {
							if file.starts_with(&file_name) {
								if prefix.is_empty() {
									prefix = file;
								} else {
									// Get common prefix
									while !file_name.starts_with(&prefix) {
										if prefix.is_empty() {
											prefix = String::new();
											break;
										}
										prefix.pop();
									}
								}
							}
						}
						if prefix.is_empty() {
							return false;
						}
						let joined = parent.join(prefix);
						self.input = self.input.clone().with_value(joined.to_str().unwrap().to_string());
					}
				}
				true
			},
			_ => {
				self.input.handle_event(&Event::Key(event));
				true
			}
		}
	}
}

impl PopupHandlePaste for InputPopup {
	fn handle_paste(&mut self, data: String) -> bool {
		self.input = self.input.clone().with_value(self.input.value().to_owned() + data.as_str());
		return true;
	}
}

impl InputPopup {
	fn complete(&self, send: bool) -> bool {
		use AwaitInput::*;
		if send {
			match self.await_input {
				AddTab => self.send_add_tab(),
				Loopback1 => self.send_loopback(true),
				Loopback2 => self.send_loopback(false),
				SetFileId => self.send_file_id(),
				SetWaveId => self.send_wave_id(),
				WaveFrequency => self.send_wave_frequency(),
				WaveAmplitude => self.send_wave_amplitude(),
				WavePhase => self.send_wave_phase(),
				WaveName => self.send_wave_name(),
				_ => (),
			}
		}
		defer_exit_popup();
		true
	}

	fn send_add_tab(&self) {
		let mut app = acquire();
		let Ok(norm) = Path::new(self.input.value()).normalize() else { return; };
		app.config.tabs.push(norm.clone().into_os_string().into_string().unwrap());
		let len = app.config.tabs.len() - 1;
		{ TabsBlock::instance().selected = len; }
		spawn_scan_thread(Scanning::One(len));
	}

	fn send_loopback(&self, one: bool) {
		let mut app = acquire();

		if one {
			app.config.loopback_1 = self.input.value().to_string();

			if !app.module_loopback_1.is_empty() {
				app.module_loopback_1 = unload_module(&app.module_loopback_1)
					.map_or(app.module_loopback_1.clone(), |_| { String::new() });

				if !app.config.loopback_1.is_empty() {
					app.module_loopback_1 = loopback(app.config.loopback_1.clone()).unwrap_or(String::new());
				}
			}
		} else {
			app.config.loopback_2 = self.input.value().to_string();

			if !app.module_loopback_2.is_empty() {
				app.module_loopback_2 = unload_module(&app.module_loopback_2)
					.map_or(app.module_loopback_2.clone(), |_| { String::new() });

				if !app.config.loopback_2.is_empty() {
					app.module_loopback_2 = loopback(app.config.loopback_2.clone()).unwrap_or(String::new());
				}
			}
		}
	}

	fn send_file_id(&self) {
		let mut app = acquire();
		let path = selected_file_path(&app.config.tabs, &app.files, None);
		if path.is_empty() {
			return;
		}
		let Ok(id) = u32::from_str_radix(self.input.value(), 10) else { return; };
		let existing = app.rev_file_id.get(&id);
		if existing.is_some() {
			if existing.unwrap() != &path {
				app.error = "File ID must be unique".to_string();
			}
			return;
		}
		app.rev_file_id.insert(id, path.clone());
		match app.config.get_file_entry_mut(path.clone()) {
			Some(entry) => {
				entry.id = Some(id);
			},
			None => {
				let mut entry = FileEntry::default();
				entry.id = Some(id);
				app.config.insert_file_entry(path, entry);
			}
		}
	}

	fn send_wave_id(&self) {
		let Ok(id) = u32::from_str_radix(self.input.value(), 10) else { return; };
		let mut app = acquire();
		let selected = { WavesBlock::instance().selected };
		app.waves[selected].id = Some(id);
		app.config.waves[selected].id = Some(id);
	}

	fn send_wave_frequency(&self) {
		let Ok(freq) = self.input.value().parse::<f32>() else { return; };
		thread::spawn(move || {
			let mut popups = popups();
			let Some(popup) = popups.iter_mut().find_map(|popup| {
				match popup {
					PopupComponent::Wave(popup) => { Option::Some(popup) },
					_ => Option::None
				}
			}) else { return; };
			let wave = &mut popup.waveform.waves[popup.selected];
			if wave.frequency != freq {
				popup.changed = true;
			}
			wave.frequency = freq;
		});
	}

	fn send_wave_amplitude(&self) {
		let Ok(amplitude) = self.input.value().parse::<f32>() else { return; };
		thread::spawn(move || {
			let mut popups = popups();
			let Some(popup) = popups.iter_mut().find_map(|popup| {
				match popup {
					PopupComponent::Wave(popup) => { Option::Some(popup) },
					_ => Option::None
				}
			}) else { return; };
			let wave = &mut popup.waveform.waves[popup.selected];
			if wave.amplitude != amplitude {
				popup.changed = true;
			}
			wave.amplitude = amplitude;
		});
	}

	fn send_wave_phase(&self) {
		let Ok(phase) = self.input.value().parse::<f32>() else { return; };
		thread::spawn(move || {
			let mut popups = popups();
			let Some(popup) = popups.iter_mut().find_map(|popup| {
				match popup {
					PopupComponent::Wave(popup) => { Option::Some(popup) },
					_ => Option::None
				}
			}) else { return; };
			let wave = &mut popup.waveform.waves[popup.selected];
			if wave.phase != phase {
				popup.changed = true;
			}
			wave.phase = phase;
		});
	}

	fn send_wave_name(&self) {
		set_wave_name(self.input.value().to_string());
	}
}