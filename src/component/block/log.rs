use std::sync::{LazyLock, Mutex, MutexGuard};

use ratatui::{style::{Color, Style}, text::Line, widgets::{Block, BorderType, Padding, Paragraph}};

use crate::{component::block::{BlockRenderArea, BlockSingleton}, state::notify_redraw};

#[allow(dead_code)]
pub enum LogLevel {
	Info,
	Warn,
	Error
}

pub struct LogBlock {
	messages: Vec<(String, LogLevel)>
}

impl BlockSingleton for LogBlock {
	fn instance() -> MutexGuard<'static, Self> {
		static BLOCK: LazyLock<Mutex<LogBlock>> = LazyLock::new(|| { Mutex::new(LogBlock {
			messages: vec![]
		}) });
		BLOCK.lock().unwrap()
	}
}

impl BlockRenderArea for LogBlock {
	fn render_area(&mut self, f: &mut ratatui::Frame, area: ratatui::prelude::Rect) {
		let block = Block::bordered()
			.border_style(Style::default().fg(Color::LightYellow))
			.border_type(BorderType::Thick)
			.padding(Padding::horizontal(1))
			.title("Log");

		let inner_height = area.height as usize - 2;
		let mut lines = vec![];
		for (body, level) in self.messages.iter().rev() {
			if lines.len() >= inner_height {
				break
			}
			lines.insert(0, Line::from(body.clone()).style(Style::default().fg(match level {
				LogLevel::Info => Color::Reset,
				LogLevel::Warn => Color::Yellow,
				LogLevel::Error => Color::Red,
			})));
		}
		f.render_widget(Paragraph::new(lines).block(block), area);
	}
}

impl LogBlock {
	fn append_message(&mut self, body: String, level: LogLevel) {
		self.messages.push((body, level));
		while self.messages.len() > 100 {
			self.messages.remove(0);
		}
		notify_redraw();
	}
}

pub fn info(body: &str) {
	LogBlock::instance().append_message(body.to_string(), LogLevel::Info);
	notify_redraw();
}

#[allow(dead_code)]
pub fn warn(body: &str) {
	LogBlock::instance().append_message(body.to_string(), LogLevel::Warn);
	notify_redraw();
}

pub fn error(body: &str) {
	LogBlock::instance().append_message(body.to_string(), LogLevel::Error);
	notify_redraw();
}