use std::{format, io::stdout, sync::{Arc, RwLock, atomic::{AtomicBool, Ordering}}, vec};

use crossterm::{execute, style::{Color::{Red, Reset, Yellow}, Print, ResetColor, SetForegroundColor}};
use ratatui::{style::{Color, Style}, text::Line, widgets::{Block, BorderType, Padding, Paragraph}};

use crate::{client::{ClientState, component::block::BlockRenderArea}, common::log::{self, LogLevel}};

pub struct LogBlock {
	flushed: Arc<AtomicBool>,
	messages: Arc<RwLock<Vec<(LogLevel, String)>>>
}

impl Default for LogBlock {
	fn default() -> Self {
		let flushed = Arc::new(AtomicBool::new(false));
		let messages = Arc::new(RwLock::new(vec![]));
		let flushed_copy = flushed.clone();
		let messages_copy = messages.clone();
		log::register(move |level, message| {
			if flushed_copy.load(Ordering::Relaxed) {
				log(&level, &message);
			} else {
				messages_copy.write().unwrap().push((level, message));
			}
		});
		Self {
			flushed,
			messages
		}
	}
}

impl BlockRenderArea for LogBlock {
	fn render_area(&mut self, _client_state: &ClientState, f: &mut ratatui::Frame, area: ratatui::prelude::Rect) {
		let block = Block::bordered()
			.border_style(Style::default().fg(Color::LightYellow))
			.border_type(BorderType::Thick)
			.padding(Padding::horizontal(1))
			.title("Log");

		let inner_height = area.height as usize - 2;
		let mut lines = vec![];
		for (level, body) in self.messages.read().unwrap().iter().rev() {
			if lines.len() >= inner_height {
				break
			}
			use LogLevel::*;
			lines.insert(0, Line::from(body.clone()).style(Style::default().fg(match level {
				Info => Color::Reset,
				Warn => Color::Yellow,
				Error => Color::Red,
			})));
		}
		f.render_widget(Paragraph::new(lines).block(block), area);
	}
}

impl LogBlock {
	pub fn flush_logs(&mut self) {
		let mut messages = self.messages.write().unwrap();
		messages.iter().for_each(|(level, message)| log(level, message));
		messages.clear();
		self.flushed.store(true, Ordering::Relaxed);
	}
}

fn log(level: &LogLevel, message: &String) {
	use LogLevel::*;
	let color = match level {
		Info => Reset,
		Warn => Yellow,
		Error => Red
	};
	let line = format!("{}\n", message);
	let _ = execute!(stdout(), SetForegroundColor(color), Print(line), ResetColor);
}