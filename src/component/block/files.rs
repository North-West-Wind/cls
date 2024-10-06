use std::{cmp::{max, min}, i32, path::Path};

use crate::{component::popup::{key_bind::{KeyBindFor, KeyBindPopup}, set_popup, PopupComponent}, state::{get_app, get_mut_app, Scanning}, util::{self, selected_file_path, threads::spawn_scan_thread}};

use super::{border_style, border_type, BlockHandleKey, BlockRenderArea};

use crossterm::event::KeyCode;
use ratatui::{layout::Rect, style::{Color, Modifier, Style}, text::{Line, Span}, widgets::{Block, Borders, Padding, Paragraph}, Frame};
use substring::Substring;

pub struct FilesBlock {
	title: String,
	id: u8,
	range: (i32, i32)
}

impl Default for FilesBlock {
	fn default() -> Self {
		Self {
			title: "Files".to_string(),
			id: 2,
			range: (-1, -1)
		}
	}
}

impl BlockRenderArea for FilesBlock {
	fn render_area(&mut self, f: &mut Frame, area: Rect) {
		let block = Block::default()
			.title(self.title.clone())
			.borders(Borders::ALL)
			.border_type(border_type(self.id))
			.border_style(border_style(self.id))
			.padding(Padding::new(2, 2, 1, 1));
	
		let app = get_mut_app();
		if self.range.0 == -1 {
			self.range = (0, area.height as i32 - 5);
		}
		let paragraph: Paragraph;
		if app.scanning == Scanning::All {
			paragraph = Paragraph::new("Performing initial scan...");
		} else if app.config.tabs.len() == 0 {
			paragraph = Paragraph::new("Add a tab to get started :>");
		} else if app.scanning == Scanning::One(app.tab_selected) {
			paragraph = Paragraph::new("Scanning this directory...\nComeback later :>");
		} else {
			let tab = app.config.tabs[app.tab_selected].clone();
			let files = app.files.as_ref().unwrap().get(&tab);
			if files.is_none() {
				paragraph = Paragraph::new("Failed to read this directory :<\nDoes it exist? Is it readable?");
			} else if files.unwrap().len() == 0 {
				paragraph = Paragraph::new("There are no playable files in this directory :<");
			} else {
				let mut lines = vec![];
				for (ii, (file, duration)) in files.unwrap().iter().enumerate() {
					let mut spans = vec![];
					if app.config.file_key.is_some() {
						let keys = app.config.file_key.as_mut().unwrap().get(&Path::new(&app.config.tabs[app.tab_selected]).join(file).into_os_string().into_string().unwrap());
						if keys.is_some() {
							let mut keys = keys.unwrap().clone();
							keys.sort();
							spans.push(Span::from(format!("({})", keys.join("+"))).style(Style::default().fg(Color::LightGreen).add_modifier(Modifier::REVERSED)));
							spans.push(Span::from(" "));
						}
					}
					let style = if app.file_selected == ii {
						Style::default().fg(Color::LightBlue).add_modifier(Modifier::REVERSED)
					} else {
						Style::default().fg(Color::Cyan)
					};
					if duration.len() == 0 {
						spans.push(Span::from(file).style(style));
					} else if file.len() + duration.len() > area.width as usize - 6 {
						let mut extra = 0;
						if spans.len() > 0 {
							extra += spans[0].width();
						}
						spans.push(Span::from(file.substring(0, area.width as usize - 10 - extra - duration.len())).style(style));
						spans.push(Span::from("... ".to_owned() + duration).style(style));
					} else {
						let mut extra = 0;
						if spans.len() > 0 {
							extra += spans[0].width() as i32;
						}
						spans.push(Span::from(file.clone()).style(style));
						spans.push(Span::from(vec![" "; max(0, area.width as i32 - 6 - extra - file.len() as i32 - duration.len() as i32) as usize].join("")).style(style));
						spans.push(Span::from(duration.clone()).style(style));
					}
					lines.push(Line::from(spans));
				}
				if app.file_selected < self.range.0 as usize {
					self.range = (app.file_selected as i32, app.file_selected as i32 + area.height as i32 - 5);
				} else if app.file_selected > self.range.1 as usize {
					self.range = (app.file_selected as i32 - area.height as i32 + 5, app.file_selected as i32);
				}
				paragraph = Paragraph::new(lines).scroll((self.range.0 as u16, 0));
			}
		}
		f.render_widget(paragraph.block(block), area);
	}
}

impl BlockHandleKey for FilesBlock {
	fn handle_key(&self, event: crossterm::event::KeyEvent) -> bool {
		let app = get_app();
		if app.scanning == Scanning::All || app.scanning == Scanning::One(app.tab_selected) {
			return false;
		}
		match event.code {
			KeyCode::Char('r') => reload_tab(),
			KeyCode::Up => navigate_file(-1),
			KeyCode::Down => navigate_file(1),
			KeyCode::Enter => play_file(),
			KeyCode::Char('x') => set_global_key_bind(),
			KeyCode::Char('z') => unset_global_key_bind(),
			KeyCode::PageUp => navigate_file(-(self.range.1 - self.range.0 + 1)),
			KeyCode::PageDown => navigate_file(self.range.1 - self.range.0 + 1),
			KeyCode::Home => navigate_file(-i32::MAX),
			KeyCode::End => navigate_file(i32::MAX),
			_ => false,
		}
	}
}

fn reload_tab() -> bool {
	let app = get_app();
	if app.tab_selected < app.config.tabs.len() {
		spawn_scan_thread(Scanning::One(app.tab_selected));
		return true;
	}
	false
}

fn navigate_file(dy: i32) -> bool {
	let app = get_mut_app();
	let files = app.files.as_ref().unwrap().get(&app.config.tabs[app.tab_selected]);
	if files.is_none() {
		return false;
	}
	let new_selected = min(files.unwrap().len() as i32 - 1, max(0, app.file_selected as i32 + dy)) as usize;
	if new_selected != app.file_selected {
		app.file_selected = new_selected;
		return true;
	}
	false
}

fn play_file() -> bool {
	let app = get_app();
	if app.files.is_none() {
		return false;
	}
	if app.tab_selected >= app.config.tabs.len() {
		return false;
	}
	let tab = app.config.tabs[app.tab_selected].clone();
	let files = app.files.as_ref().unwrap().get(&tab);
	if files.is_none() {
		return false;
	}
	let unwrapped = files.unwrap();
	if app.file_selected >= unwrapped.len() {
		return false;
	}
	util::pulseaudio::play_file(&Path::new(&tab).join(&unwrapped[app.file_selected].0).into_os_string().into_string().unwrap());
	return true;
}

fn set_global_key_bind() -> bool {
	set_popup(PopupComponent::KeyBind(KeyBindPopup::new(KeyBindFor::File)));
	return true;
}

fn unset_global_key_bind() -> bool {
	let path = selected_file_path();
	if path.is_empty() {
		return false;
	}
	let app = get_mut_app();
	if app.config.file_key.is_none() {
		return false;
	}
	app.config.file_key.as_mut().unwrap().remove(&path);
	app.hotkey.as_mut().unwrap().remove(&path);
	return true;
}