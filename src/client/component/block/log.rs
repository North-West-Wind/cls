use std::{format, io::stdout, sync::{Arc, atomic::{AtomicBool, Ordering}}, vec};

use crossterm::{execute, style::{Color::{Red, Reset, Yellow}, Print, ResetColor, SetForegroundColor}};
use parking_lot::RwLock;
use ratatui::{style::{Color, Style}, text::Line, widgets::{Block, BorderType, Padding, Paragraph}};
use rayon::iter::{IndexedParallelIterator, IntoParallelRefIterator, ParallelIterator};

use crate::{client::{ClientState, Redrawer, component::block::BlockRenderArea}, common::log::{self, LogLevel}};

pub struct LogBlock {
	flushed: Arc<AtomicBool>,
	messages: Arc<RwLock<Vec<(LogLevel, String)>>>,
}

impl BlockRenderArea for LogBlock {
	fn render_area(&mut self, _client_state: &ClientState, f: &mut ratatui::Frame, area: ratatui::prelude::Rect) {
		let block = Block::bordered()
			.border_style(Style::default().fg(Color::LightYellow))
			.border_type(BorderType::Thick)
			.padding(Padding::horizontal(1))
			.title("Log");

		let inner_height = area.height as usize - 2;
		let messages = self.messages.read();
		let start = messages.len().saturating_sub(inner_height);
		let lines = self.messages.read()[start..].par_iter().rev().map(|(level, body)| {
			use LogLevel::*;
			Line::from(body.clone()).style(Style::default().fg(match level {
				Info => Color::Reset,
				Warn => Color::Yellow,
				Error => Color::Red,
			}))
		}).collect::<Vec<_>>();
		f.render_widget(Paragraph::new(lines).block(block), area);
	}
}

impl LogBlock {
	pub fn new(redrawer: Redrawer) -> Self {
		let flushed = Arc::new(AtomicBool::new(false));
		let messages = Arc::new(RwLock::new(vec![]));
		let flushed_copy = flushed.clone();
		let messages_copy = messages.clone();
		log::register(move |level, message| {
			if flushed_copy.load(Ordering::Relaxed) {
				log(&level, &message);
			} else {
				messages_copy.write().push((level, message));
				redrawer.notify();
			}
		});
		Self {
			flushed,
			messages
		}
	}

	pub fn flush_logs(&mut self) {
		let mut messages = self.messages.write();
		// Need order, don't use par_iter
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