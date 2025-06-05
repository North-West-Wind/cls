use std::{cmp::{max, min}, collections::HashSet, i32, path::Path};

use crate::{component::{block::{borders, settings::SettingsBlock, tabs::TabsBlock, BlockNavigation}, popup::{input::{AwaitInput, InputPopup}, key_bind::{KeyBindFor, KeyBindPopup}, set_popup, PopupComponent}}, state::{config, config_mut, get_app, get_mut_app, Scanning}, util::{self, selected_file_path, threads::spawn_scan_thread}};

use super::{loop_index, BlockHandleKey, BlockRenderArea};

use crossterm::event::KeyCode;
use rand::Rng;
use ratatui::{layout::Rect, style::{Color, Modifier, Style}, text::{Line, Span}, widgets::{Block, Borders, Padding, Paragraph, Wrap}, Frame};
use substring::Substring;

pub struct FilesBlock {
	range: (i32, i32),
	pub(super) selected: usize,
}

impl Default for FilesBlock {
	fn default() -> Self {
		Self {
			range: (-1, -1),
			selected: 0,
		}
	}
}

impl BlockRenderArea for FilesBlock {
	fn render_area(&mut self, f: &mut Frame, area: Rect) {
		let (border_type, border_style) = borders(Self::ID);
		let block = Block::default()
			.title("Files")
			.borders(Borders::ALL)
			.border_type(border_type)
			.border_style(border_style)
			.padding(Padding::new(2, 2, 1, 1));
	
		let app = get_mut_app();
		if self.range.0 == -1 {
			self.range = (0, area.height as i32 - 5);
		}
		let config = config();
		let paragraph: Paragraph;
		if app.scanning == Scanning::All {
			paragraph = Paragraph::new("Performing initial scan...").wrap(Wrap { trim: false });
		} else if config.tabs.len() == 0 {
			paragraph = Paragraph::new("Add a tab to get started :>").wrap(Wrap { trim: false });
		} else if app.scanning == Scanning::One(app.tab_selected()) {
			paragraph = Paragraph::new("Scanning this directory...\nComeback later :>").wrap(Wrap { trim: false });
		} else {
			let tab = config.tabs[app.tab_selected()].clone();
			let files = app.files.get(&tab);
			paragraph = files.map_or_else(|| {
				Paragraph::new("Failed to read this directory :<\nDoes it exist? Is it readable?").wrap(Wrap { trim: false })
			}, |files| {
				if files.len() == 0 {
					return Paragraph::new("There are no playable files in this directory :<").wrap(Wrap { trim: false });
				}
				let mut lines = vec![];
				for (ii, (file, duration)) in files.iter().enumerate() {
					let mut spans = vec![];
					let tab_selected = app.tab_selected();
					let full_path = &Path::new(&config.tabs[tab_selected]).join(file).into_os_string().into_string().unwrap();
					let entry = config.get_file_entry(full_path.clone());
					entry.inspect(|entry| {
						entry.id.inspect(|id| {
							spans.push(Span::from(format!("({})", id)).style(Style::default().fg(Color::LightYellow).add_modifier(Modifier::REVERSED)));
							spans.push(Span::from(" "));
						});
						if entry.keys.len() > 0 {
							let mut keys = Vec::from_iter(entry.keys.clone().into_iter());
							keys.sort();
							spans.push(Span::from(format!("{{{}}}", keys.join(" "))).style(Style::default().fg(Color::LightGreen).add_modifier(Modifier::REVERSED)));
							spans.push(Span::from(" "));
						}
					});
					let style = if self.selected == ii {
						Style::default().fg(Color::LightBlue).add_modifier(Modifier::REVERSED)
					} else {
						Style::default().fg(Color::Cyan)
					};
					let extra = spans.iter()
						.map(|span| { span.width() as i32 })
						.fold(0, |acc, width| { acc + width });
					if file.len() + duration.len() + extra as usize > area.width as usize - 6 {
						spans.push(Span::from(file.substring(0, max(0, area.width as i32 - 10 - extra - duration.len() as i32) as usize)).style(style));
						spans.push(Span::from("... ".to_owned() + duration).style(style));
					} else {
						spans.push(Span::from(file.clone()).style(style));
						spans.push(Span::from(vec![" "; max(0, area.width as i32 - 6 - extra - file.len() as i32 - duration.len() as i32) as usize].join("")).style(style));
						spans.push(Span::from(duration.clone()).style(style));
					}
					lines.push(Line::from(spans));
				}
				if self.selected < self.range.0 as usize {
					self.range = (self.selected as i32, self.selected as i32 + area.height as i32 - 5);
				} else if self.selected > self.range.1 as usize {
					self.range = (self.selected as i32 - area.height as i32 + 5, self.selected as i32);
				}
				Paragraph::new(lines).scroll((self.range.0 as u16, 0))
			});
		}
		f.render_widget(paragraph.block(block), area);
	}
}

impl BlockHandleKey for FilesBlock {
	fn handle_key(&mut self, event: crossterm::event::KeyEvent) -> bool {
		let app = get_app();
		if app.scanning == Scanning::All || app.scanning == Scanning::One(self.selected) {
			return false;
		}
		match event.code {
			KeyCode::Char('r') => self.reload_tab(),
			KeyCode::Up => self.navigate_file(-1),
			KeyCode::Down => self.navigate_file(1),
			KeyCode::Enter => self.play_file(false),
			KeyCode::Char('/') => self.play_file(true),
			KeyCode::Char('x') => self.set_global_key_bind(),
			KeyCode::Char('z') => self.unset_global_key_bind(),
			KeyCode::Char('v') => self.set_file_id(),
			KeyCode::Char('b') => self.unset_file_id(),
			KeyCode::PageUp => self.navigate_file(-(self.range.1 - self.range.0 + 1)),
			KeyCode::PageDown => self.navigate_file(self.range.1 - self.range.0 + 1),
			KeyCode::Home => self.navigate_file(-i32::MAX),
			KeyCode::End => self.navigate_file(i32::MAX),
			_ => false,
		}
	}
}

impl BlockNavigation for FilesBlock {
	const ID: u8 = 2;

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

impl FilesBlock {
	fn play_file(&self, random: bool) -> bool {
		let app = get_app();
		let selected = app.tab_selected();
		let config = config();
		if selected >= config.tabs.len() {
			return false;
		}
		let tab = config.tabs[selected].clone();
		let files = app.files.get(&tab);
		return files.map_or_else(|| { false }, |files| {
			let index;
			if random {
				index = rand::thread_rng().gen_range(0..files.len());
			} else {
				if self.selected >= files.len() {
					return false;
				}
				index = self.selected;
			}
			util::pulseaudio::play_file(&Path::new(&tab).join(&files[index].0).into_os_string().into_string().unwrap());
			true
		});
	}

	fn navigate_file(&mut self, dy: i32) -> bool {
		let app = get_app();
		let files = app.files.get(&config().tabs[app.tab_selected()]);
		return files.map_or(false, |files| {
			let files = files.len();
			let new_selected;
			if dy.abs() > 1 {
				new_selected = min(files as i32 - 1, max(0, self.selected as i32 + dy)) as usize;
			} else {
				new_selected = loop_index(self.selected, dy, files);
			}
			if new_selected != self.selected {
				self.selected = new_selected;
				return true;
			}
			false
		});
	}

	fn reload_tab(&self) -> bool {
		if self.selected < config().tabs.len() {
			spawn_scan_thread(Scanning::One(self.selected));
			return true;
		}
		false
	}

	fn set_global_key_bind(&self) -> bool {
		let path = selected_file_path();
		if path.is_empty() {
			return false;
		}
		let app = get_app();
		let hotkey = app.hotkey.get(&path);
		let recorded = match hotkey {
			Option::Some(vec) => HashSet::from_iter(vec.iter().map(|key| { *key })),
			Option::None => HashSet::new(),
		};
		set_popup(PopupComponent::KeyBind(KeyBindPopup::new(KeyBindFor::File, recorded)));
		return true;
	}

	fn unset_global_key_bind(&self) -> bool {
		let path = selected_file_path();
		if path.is_empty() {
			return false;
		}
		let config = config_mut();
		let entry = config.get_file_entry_mut(path.clone());
		return entry.map_or_else(|| { false }, |entry| {
			if entry.keys.is_empty() {
				return false;
			}
			entry.keys.clear();
			get_mut_app().hotkey.remove(&path);
			true
		});
	}

	fn set_file_id(&self) -> bool {
		let path = selected_file_path();
		if path.is_empty() {
			return false;
		}
		let init = match config().get_file_entry(path) {
			Some(entry) => match entry.id {
				Some(id) => id.to_string(),
				None => String::new(),
			},
			None => String::new(),
		};
		set_popup(PopupComponent::Input(InputPopup::new(init, AwaitInput::SetFileId)));
		return true;
	}

	fn unset_file_id(&self) -> bool {
		let path = selected_file_path();
		if path.is_empty() {
			return false;
		}
		let app = get_mut_app();
		let entry = config_mut().get_file_entry_mut(path);
		return entry.map_or_else(|| { false }, |entry| {
			let id = entry.id;
			return id.map_or_else(|| { false }, |id| {
				app.rev_file_id.remove(&id);
				entry.id = Option::None;
				true
			});
		});
	}
}

