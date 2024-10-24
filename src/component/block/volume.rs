use std::{cmp::{max, min}, collections::HashMap};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{layout::Rect, style::{Color, Modifier, Style}, text::{Line, Span, Text}, widgets::{Block, Borders, Padding, Paragraph}, Frame};

use crate::{util::pulseaudio::set_volume_percentage, state::{get_app, get_mut_app}, util::selected_file_path};

use super::{border_style, border_type, BlockHandleKey, BlockRenderArea};

pub struct VolumeBlock {
	title: String,
	id: u8,
}

impl Default for VolumeBlock {
	fn default() -> Self {
		Self {
			title: "Volume".to_string(),
			id: 0
		}
	}
}

impl BlockRenderArea for VolumeBlock {
	fn render_area(&mut self, f: &mut Frame, area: Rect) {
		let app = get_app();
		let block = Block::default()
			.title(self.title.clone())
			.borders(Borders::ALL)
			.border_type(border_type(self.id))
			.border_style(border_style(self.id))
			.padding(Padding::horizontal(1));
		let mut lines = vec![
			volume_line("Sink Volume".to_string(), app.config.volume as usize, area.width, app.volume_selected == 0)
		];
		let path = selected_file_path();
		if !path.is_empty() {
			lines.push(Line::from(""));
			lines.push(Line::from(vec![
				Span::from("Selected "),
				Span::from(path.clone()).style(Style::default().fg(Color::LightGreen))
			]));
			let mut volume = 100;
			let file_volume = app.config.file_volume.as_ref();
			if file_volume.is_some() {
				let val = file_volume.unwrap().get(&path);
				if val.is_some() {
					volume = *val.unwrap();
				}
			}
			lines.push(volume_line("File Volume".to_string(), volume, area.width, app.volume_selected == 1));
		}
		let paragraph = Paragraph::new(Text::from(lines))
			.block(block);
		f.render_widget(paragraph, area);
	}
}

impl BlockHandleKey for VolumeBlock {
	fn handle_key(&mut self, event: KeyEvent) -> bool {
		match event.code {
			KeyCode::Right => change_volume(if event.modifiers.contains(KeyModifiers::CONTROL) { 5 } else { 1 }),
			KeyCode::Left => change_volume(if event.modifiers.contains(KeyModifiers::CONTROL) { -5 } else { -1 }),
			KeyCode::Up => select_volume(0),
			KeyCode::Down => select_volume(1),
			_ => false
		}
	}
}

fn volume_line(title: String, volume: usize, width: u16, highlight: bool) -> Line<'static> {
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

fn select_volume(selection: usize) -> bool {
	let app = get_mut_app();
	if app.volume_selected != selection {
		if selection == 1 {
			let selected_file = selected_file_path();
			if selected_file.is_empty() {
				return false;
			}
		}
		app.volume_selected = selection;
		return true;
	}
	false
}

fn change_volume(delta: i16) -> bool {
	let app = get_mut_app();
	if app.volume_selected == 1 {
		return change_file_volume(delta);
	}
	let old_volume = app.config.volume as i16;
	let new_volume = min(200, max(0, old_volume + delta));
	if new_volume != old_volume {
		set_volume_percentage(new_volume as u32);
		app.config.volume = new_volume as u32;
		return true
	}
	false
}

fn change_file_volume(delta: i16) -> bool {
	let selected_file = selected_file_path();
	if selected_file.is_empty() {
		return false;
	}
	let app = get_mut_app();
	if app.config.file_volume.is_none() {
		app.config.file_volume = Option::Some(HashMap::new());
	}
	let map = app.config.file_volume.as_mut().unwrap();
	let old_volume = map.get(&selected_file).unwrap_or(&100);
	let new_volume = min(100, max(0, (*old_volume) as i16 + delta)) as usize;
	if new_volume != *old_volume {
		map.insert(selected_file, new_volume);
		return true
	}
	false
}