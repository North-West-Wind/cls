use std::{cmp::{max, min}, sync::{LazyLock, Mutex, MutexGuard}};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{layout::Rect, style::{Color, Modifier, Style}, text::{Line, Span, Text}, widgets::{Block, Borders, Padding, Paragraph}, Frame};

use crate::{component::block::{BlockNavigation, BlockSingleton, tabs::TabsBlock, waves::WavesBlock}, config::FileEntry, state::acquire, util::{global_input::{keyboard_to_string, sort_keys}, pulseaudio::set_volume_percentage, selected_file_path}};

use super::{loop_index, BlockHandleKey, BlockRenderArea};

pub struct InfoBlock {
	selected: usize,
	options: u8,
}

impl BlockSingleton for InfoBlock {
	fn instance() -> MutexGuard<'static, Self> {
		static BLOCK: LazyLock<Mutex<InfoBlock>> = LazyLock::new(|| { Mutex::new(InfoBlock {
			selected: 0,
			options: 2
		}) });
		BLOCK.lock().unwrap()
	}
}

impl BlockRenderArea for InfoBlock {
	fn render_area(&mut self, f: &mut Frame, area: Rect) {
		let app = acquire();
		let (border_type, border_style) = app.borders(Self::ID);
		let block = Block::default()
			.title("Volume")
			.borders(Borders::ALL)
			.border_type(border_type)
			.border_style(border_style)
			.padding(Padding::horizontal(1));
		let mut lines = vec![
			volume_line("Sink Volume".to_string(), app.config.volume, area.width, self.selected == 0)
		];
		if app.waves_opened {
			let index = { WavesBlock::instance().selected };
			if index < app.waves.len() {
				let wave = &app.waves[index];
				lines.push(Line::from(""));
				lines.push(Line::from(vec![
					Span::from("Selected "),
					Span::from(format!("{} ({})", wave.label, wave.details())).style(Style::default().fg(Color::LightBlue))
				]));
				lines.push(volume_line("Wave Volume".to_string(), wave.volume, area.width, self.selected == 1));
				let mut spans = vec![];
				spans.push(Span::from("ID "));
				spans.push(wave.id.map_or( Span::from("None").style(Style::default().fg(Color::Red)), |id| { Span::from(format!(" {} ", id)).style(Style::default().fg(Color::LightYellow).add_modifier(Modifier::REVERSED)) }));
				spans.push(Span::from(" | Keys "));
				if wave.keys.is_empty() {
					spans.push(Span::from("None").style(Style::default().fg(Color::Red)));
				} else {
					let mut keys = wave.keys.iter().map(|key| keyboard_to_string(*key)).collect::<Vec<String>>();
					let keys = sort_keys(&mut keys);
					spans.push(Span::from(format!(" {{{}}} ", keys.join(" "))).style(Style::default().fg(Color::LightGreen).add_modifier(Modifier::REVERSED)));
				}
				lines.push(Line::from(spans));
			}
		} else {
			let path = selected_file_path(&app.config.tabs, &app.files, None);
			if !path.is_empty() {
				lines.push(Line::from(""));
				lines.push(Line::from(vec![
					Span::from("Selected "),
					Span::from(path.clone()).style(Style::default().fg(Color::LightGreen))
				]));
				let (volume, hotkey, file_id) = match app.config.get_file_entry(path) {
					Some(entry) => (entry.volume, if entry.keys.is_empty() { None } else {
						let mut keys = entry.keys.clone().into_iter().collect::<Vec<String>>();
						let keys = sort_keys(&mut keys);
						Some(format!("{{{}}}", keys.join(" ")))
					}, entry.id),
					None => (100, None, None)
				};
				lines.push(volume_line("File Volume".to_string(), volume, area.width, self.selected == 1));
				let mut spans = vec![];
				spans.push(Span::from("ID "));
				spans.push(file_id.map_or( Span::from("None").style(Style::default().fg(Color::Red)), |id| { Span::from(format!(" {} ", id)).style(Style::default().fg(Color::LightYellow).add_modifier(Modifier::REVERSED)) }));
				spans.push(Span::from(" | Keys "));
				spans.push(hotkey.map_or(Span::from("None").style(Style::default().fg(Color::Red)), |keys| { Span::from(format!(" {} ", keys)).style(Style::default().fg(Color::LightGreen).add_modifier(Modifier::REVERSED)) }));
				lines.push(Line::from(spans));
			}
		}
		let paragraph = Paragraph::new(Text::from(lines))
			.block(block);
		f.render_widget(paragraph, area);
	}
}

impl BlockHandleKey for InfoBlock {
	fn handle_key(&mut self, event: KeyEvent) -> bool {
		match event.code {
			KeyCode::Right => self.change_volume(if event.modifiers.contains(KeyModifiers::CONTROL) { 5 } else { 1 }),
			KeyCode::Left => self.change_volume(if event.modifiers.contains(KeyModifiers::CONTROL) { -5 } else { -1 }),
			KeyCode::Up => self.navigate_volume(-1),
			KeyCode::Down => self.navigate_volume(1),
			_ => false
		}
	}
}

impl BlockNavigation for InfoBlock {
	const ID: u8 = 0;

	fn navigate_block(&self, _dx: i16, dy: i16) -> u8 {
		if dy > 0 {
			return TabsBlock::ID;
		}
		return Self::ID;
	}
}

impl InfoBlock {
	fn navigate_volume(&mut self, dy: i32) -> bool {
		let new_selected = loop_index(self.selected, dy, self.options as usize);
		if new_selected != self.selected {
			if new_selected == 1 {
				let app = acquire();
				let selected_file = selected_file_path(&app.config.tabs, &app.files, None);
				if selected_file.is_empty() {
					return false;
				}
			}
			self.selected = new_selected;
			return true;
		}
		false
	}

	fn change_volume(&self, delta: i16) -> bool {
		let waves_opened = { acquire().waves_opened };
		if self.selected == 1 {
			if waves_opened {
				return change_wave_volume(delta);
			} else {
				return change_file_volume(delta);
			}
		}
		let mut app = acquire();
		let old_volume = app.config.volume as i16;
		let new_volume = min(200, max(0, old_volume + delta));
		if new_volume != old_volume {
			set_volume_percentage(new_volume as u32);
			app.config.volume = new_volume as u32;
			return true
		}
		false
	}
}

fn volume_line(title: String, volume: u32, width: u16, highlight: bool) -> Line<'static> {
	let mut spans = vec![];
	spans.push(Span::from(title).style(if highlight { Style::default().fg(Color::LightCyan).add_modifier(Modifier::REVERSED) } else { Style::default() }));
	spans.push(Span::from(format!(" ({:0>3}%) ", volume)));
	let verticals: usize;
	let full: usize;
	if width >= 122 {
		verticals = min(volume as usize, 100);
		full = 100;
	} else if width >= 72 {
		verticals = min(volume as usize, 100) / 2;
		full = 50;
	} else {
		verticals = min(volume as usize, 100) / 5;
		full = 20;
	}
	spans.push(Span::from(vec!["|"; verticals].join("")).style(Style::default().fg(if volume > 100 {
		Color::Red
	} else {
		Color::LightGreen
	})));
	spans.push(Span::from(vec!["-"; full - verticals].join("")).style(Style::default().fg(if volume > 100 {
		Color::Red
	} else {
		Color::Green
	})));
	Line::from(spans)
}

fn change_file_volume(delta: i16) -> bool {
	let mut app = acquire();
	let path = selected_file_path(&app.config.tabs, &app.files, None);
	if path.is_empty() {
		return false;
	}
	match app.config.get_file_entry_mut(path.clone()) {
		Some(entry) => {
			let old_volume = entry.volume;
			let new_volume = min(100, max(0, old_volume as i16 + delta)) as u32;
			if new_volume != old_volume {
				entry.volume = new_volume;
				if entry.is_default() {
					app.config.remove_file_entry(path);
				}
				return true;
			}
		},
		None => {
			let mut entry = FileEntry::default();
			entry.volume = (100 + delta) as u32;
			app.config.insert_file_entry(path, entry);
			return true;
		}
	};
	false
}

fn change_wave_volume(delta: i16) -> bool {
	let mut app = acquire();
	let index = { WavesBlock::instance().selected };
	if index >= app.waves.len() {
		return false;
	}
	let wave = &mut app.waves[index];
	let new_volume = min(100, max(0, wave.volume as i16 + delta)) as u32;
	if new_volume != wave.volume {
		wave.volume = new_volume;
		app.config.waves[index].volume = new_volume;
		return true;
	}
	false
}