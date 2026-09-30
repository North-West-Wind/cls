use std::{cmp::{max, min}, format, path::Path, vec};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{layout::Rect, style::{Color, Modifier, Style}, text::{Line, Span, Text}, widgets::{Block, Borders, Padding, Paragraph}, Frame};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

use crate::{client::{AtomicClientState, ClientState, MainOpened, SearchResult, component::block::{BlockNavigation, search::SearchBlock, tabs::TabsBlock}}, common::keyboard::{keyboard_to_string, sort_keys}};

use super::{loop_index, BlockHandleKey, BlockRenderArea};

pub struct InfoBlock {
	selected: usize,
	options: u8,
}

impl Default for InfoBlock {
	fn default() -> Self {
		Self {
			selected: 0,
			options: 2
		}
	}
}

impl BlockRenderArea for InfoBlock {
	fn render_area(&mut self, client_state: &ClientState, f: &mut Frame, area: Rect) {
		let (border_type, border_style) = client_state.borders(Self::ID);
		let block = Block::default()
			.title("Volume")
			.borders(Borders::ALL)
			.border_type(border_type)
			.border_style(border_style)
			.padding(Padding::horizontal(1));
		let mut lines = vec![
			volume_line("Sink Volume".to_string(), client_state.config.volume, area.width, self.selected == 0)
		];
		use MainOpened::*;
		match client_state.main_opened {
			File => {
				if let Some((parent, name, info)) = client_state.get_file() {
					lines.push(Line::from(""));
					lines.push(Line::from(vec![
						Span::from("Selected "),
						Span::from(Path::new(&parent).join(name).to_str().unwrap().to_string()).style(Style::default().fg(Color::LightGreen))
					]));
					let volume = info.base.volume;
					let hotkey = if info.base.keys.is_empty() {
						None
					} else {
						let mut keys = info.base.keys.clone().into_iter().collect::<Vec<String>>();
						let keys = sort_keys(&mut keys);
						Some(format!("{{{}}}", keys.join(" ")))
					};
					let file_id = info.base.id;
					lines.push(volume_line("File Volume".to_string(), volume, area.width, self.selected == 1));
					let mut spans = vec![];
					spans.push(Span::from("ID "));
					spans.push(file_id.map_or( Span::from("None").style(Style::default().fg(Color::Red)), |id| { Span::from(format!(" {} ", id)).style(Style::default().fg(Color::LightYellow).add_modifier(Modifier::REVERSED)) }));
					spans.push(Span::from(" | Keys "));
					spans.push(hotkey.map_or(Span::from("None").style(Style::default().fg(Color::Red)), |keys| { Span::from(format!(" {} ", keys)).style(Style::default().fg(Color::LightGreen).add_modifier(Modifier::REVERSED)) }));
					lines.push(Line::from(spans));
				}
			},
			Wave => {
				let index = client_state.selected_wave;
				if index < client_state.waves.len() {
					let wave = &client_state.waves[index];
					lines.push(Line::from(""));
					lines.push(Line::from(vec![
						Span::from("Selected "),
						Span::from(format!("{} ({})", wave.base.label, wave.details())).style(Style::default().fg(Color::LightBlue))
					]));
					lines.push(volume_line("Wave Volume".to_string(), wave.base.volume, area.width, self.selected == 1));
					let mut spans = vec![];
					spans.push(Span::from("ID "));
					spans.push(wave.base.id.map_or( Span::from("None").style(Style::default().fg(Color::Red)), |id| { Span::from(format!(" {} ", id)).style(Style::default().fg(Color::LightYellow).add_modifier(Modifier::REVERSED)) }));
					spans.push(Span::from(" | Keys "));
					if wave.base.keys.is_empty() {
						spans.push(Span::from("None").style(Style::default().fg(Color::Red)));
					} else {
						let mut keys = wave.base.keys.par_iter().map(|key| keyboard_to_string(*key)).collect::<Vec<String>>();
						let keys = sort_keys(&mut keys);
						spans.push(Span::from(format!(" {{{}}} ", keys.join(" "))).style(Style::default().fg(Color::LightGreen).add_modifier(Modifier::REVERSED)));
					}
					lines.push(Line::from(spans));
				}
			},
			Dialog => {
				let index = client_state.selected_dialog;
				if index < client_state.dialogs.len() {
					let dialog = &client_state.dialogs[index];
					lines.push(Line::from(""));
					lines.push(Line::from(vec![
						Span::from("Selected "),
						Span::from(dialog.label.clone()).style(Style::default().fg(Color::LightYellow))
					]));
					lines.push(volume_line("Dialog Volume".to_string(), dialog.volume, area.width, self.selected == 1));
					let mut spans = vec![];
					spans.push(Span::from("ID "));
					spans.push(dialog.id.map_or( Span::from("None").style(Style::default().fg(Color::Red)), |id| { Span::from(format!(" {} ", id)).style(Style::default().fg(Color::LightYellow).add_modifier(Modifier::REVERSED)) }));
					spans.push(Span::from(" | Keys "));
					if dialog.keys.is_empty() {
						spans.push(Span::from("None").style(Style::default().fg(Color::Red)));
					} else {
						let mut keys = dialog.keys.par_iter().map(|key| keyboard_to_string(*key)).collect::<Vec<String>>();
						let keys = sort_keys(&mut keys);
						spans.push(Span::from(format!(" {{{}}} ", keys.join(" "))).style(Style::default().fg(Color::LightGreen).add_modifier(Modifier::REVERSED)));
					}
					lines.push(Line::from(spans));
				}
			},
			Search => {
				use SearchResult::*;
				if client_state.search_results.len() > 0 && client_state.selected_result < client_state.search_results.len() {
					let (name, volume, keys, id, volume_label) = match &client_state.search_results[client_state.selected_result].1 {
						File(result) => {
							let keys = if result.info.base.keys.is_empty() { None } else {
								let mut keys: Vec<String> = result.info.base.keys.clone().into_iter().collect();
								let keys = sort_keys(&mut keys);
								Some(format!("{{{}}}", keys.join(" ")))
							};
							(result.name.clone(), result.info.base.volume, keys, result.info.base.id, "File")
						},
						Wave(result) => {
							client_state.waves.par_iter().find_any(|wave| wave.base.uid == result.uid).map_or((String::new(), 0, None, None, "Wave"), |wave| {
								(format!("{} ({})", result.main, result.sub), wave.base.volume, if wave.base.keys.is_empty() { None } else {
									let mut keys = wave.base.keys.par_iter().map(|key| keyboard_to_string(*key)).collect::<Vec<String>>();
									let keys = sort_keys(&mut keys);
									Some(format!("{{{}}}", keys.join(" ")))
								}, wave.base.id, "Wave")
							})
						},
						Dialog(result) => {
							client_state.dialogs.par_iter().find_any(|dialog| dialog.uid == result.uid).map_or((String::new(), 0, None, None, "Dialog"), |dialog| {
								(result.main.clone(), dialog.volume, if dialog.keys.is_empty() { None } else {
									let mut keys = dialog.keys.par_iter().map(|key| keyboard_to_string(*key)).collect::<Vec<String>>();
									let keys = sort_keys(&mut keys);
									Some(format!("{{{}}}", keys.join(" ")))
								}, dialog.id, "Dialog")
							})
						}
					};
					lines.push(Line::from(""));
					lines.push(Line::from(vec![
						Span::from("Selected "),
						Span::from(name).style(Style::default().fg(Color::LightGreen))
					]));
					lines.push(volume_line(format!("{volume_label} Volume"), volume, area.width, self.selected == 1));
					let mut spans = vec![];
					spans.push(Span::from("ID "));
					spans.push(id.map_or( Span::from("None").style(Style::default().fg(Color::Red)), |id| { Span::from(format!(" {} ", id)).style(Style::default().fg(Color::LightYellow).add_modifier(Modifier::REVERSED)) }));
					spans.push(Span::from(" | Keys "));
					spans.push(keys.map_or(Span::from("None").style(Style::default().fg(Color::Red)), |keys| { Span::from(format!(" {} ", keys)).style(Style::default().fg(Color::LightGreen).add_modifier(Modifier::REVERSED)) }));
					lines.push(Line::from(spans));
				}
			},
			_ => ()
		}
		let paragraph = Paragraph::new(Text::from(lines))
			.block(block);
		f.render_widget(paragraph, area);
	}
}

impl BlockHandleKey for InfoBlock {
	fn handle_key(&mut self, client_state: AtomicClientState, event: KeyEvent) -> bool {
		match event.code {
			KeyCode::Right => self.change_volume(client_state, if event.modifiers.contains(KeyModifiers::CONTROL) { 5 } else { 1 }),
			KeyCode::Left => self.change_volume(client_state, if event.modifiers.contains(KeyModifiers::CONTROL) { -5 } else { -1 }),
			KeyCode::Up => self.navigate_volume(-1),
			KeyCode::Down => self.navigate_volume(1),
			_ => false
		}
	}
}

impl BlockNavigation for InfoBlock {
	const ID: u8 = 0;

	fn navigate_block(&self, client_state: AtomicClientState, _dx: i16, dy: i16) -> u8 {
		if dy > 0 {
			if client_state.read().main_opened == MainOpened::Search {
				return SearchBlock::ID;
			} else {
				return TabsBlock::ID;
			}
		}
		return Self::ID;
	}
}

impl InfoBlock {
	fn navigate_volume(&mut self, dy: i32) -> bool {
		let new_selected = loop_index(self.selected, dy, self.options as usize);
		if new_selected != self.selected {
			self.selected = new_selected;
			return true;
		}
		false
	}

	fn change_volume(&self, client_state: AtomicClientState, delta: i64) -> bool {
		if self.selected == 1 {
			return match { client_state.read().main_opened } {
				MainOpened::File => change_file_volume(client_state, delta),
				MainOpened::Wave => change_wave_volume(client_state, delta),
				MainOpened::Dialog => change_dialog_volume(client_state, delta),
				_ => false
			};
		}
		let mut client_state = client_state.write();
		let old_volume = client_state.config.volume as i64;
		let new_volume = max(0, old_volume + delta);
		if new_volume != old_volume {
			client_state.config.volume = new_volume as u32;
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

fn change_file_volume(client_state: AtomicClientState, delta: i64) -> bool {
	let mut client_state = client_state.write();
	let (selected_tab, selected_file) = (client_state.selected_tab, client_state.selected_file);
	let (_, files) = &mut client_state.file_tabs[selected_tab];
	let file = &mut files[selected_file];

	let old_volume = file.base.volume;
	let new_volume = max(0, old_volume as i64 + delta) as u32;
	if new_volume != old_volume {
		file.base.volume = new_volume;
		return true;
	}
	false
}

fn change_wave_volume(client_state: AtomicClientState, delta: i64) -> bool {
	let mut client_state = client_state.write();
	let selected_wave = client_state.selected_wave;
	
	let wave = &mut client_state.waves[selected_wave];
	let new_volume = max(0, wave.base.volume as i64 + delta) as u32;
	if new_volume != wave.base.volume {
		wave.base.volume = new_volume;
		return true;
	}
	false
}

fn change_dialog_volume(client_state: AtomicClientState, delta: i64) -> bool {
	let mut client_state = client_state.write();
	let selected_dialog = client_state.selected_dialog;
	
	let dialog = &mut client_state.dialogs[selected_dialog];
	let new_volume = max(0, dialog.volume as i64 + delta) as u32;
	if new_volume != dialog.volume {
		dialog.volume = new_volume;
		return true;
	}
	false
}