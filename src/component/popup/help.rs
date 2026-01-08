use std::cmp::{max, min};

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{style::{Modifier, Style}, text::{Line, Text}, widgets::{Block, BorderType, Clear, Padding, Paragraph, Widget}, Frame};

use crate::{component::popup::defer_exit_popup, constant::{APP_NAME, APP_VERSION}};

use super::{safe_centered_rect, PopupHandleKey, PopupRender};

pub struct HelpPopup {
	page: u8,
	max_page: u8,
}

impl Default for HelpPopup {
	fn default() -> Self {
		Self {
			page: 1,
			max_page: 3,
		}
	}
}

impl PopupRender for HelpPopup {
	fn render(&self, f: &mut Frame) {
		let appname = APP_NAME;
		let mut lines = vec![
			Line::from(format!("{appname} - Command Line Soundboard")).style(Style::default().add_modifier(Modifier::BOLD)).centered(),
			Line::from(APP_VERSION).style(Style::default().add_modifier(Modifier::BOLD)).centered(),
			Line::from(format!("Page {} / {}", self.page, self.max_page)).centered(),
			Line::from(""),
		];

		match self.page {
			1 => {
				lines.extend(vec![
					Line::from("Root Key Binds").style(Style::default().add_modifier(Modifier::BOLD)).centered(),
					Line::from("? - Help"),
					Line::from("q / esc - Escape / Quit"),
					Line::from("arrow keys - Navigate"),
					Line::from("enter - Select block"),
					Line::from("c - Toggle settings menu"),
					Line::from("w - Toggle wave menu"),
					Line::from("\\ - Toggle logs"),
					Line::from("s - Save configuration"),

					Line::from(""),
					Line::from("Help Key Binds").style(Style::default().add_modifier(Modifier::BOLD)).centered(),
					Line::from("left / right - Change page"),
				]);
			}
			2 => {
				lines.extend(vec![
					Line::from("Volume Key Binds").style(Style::default().add_modifier(Modifier::BOLD)).centered(),
					Line::from("left - Decrease volume by 1%"),
					Line::from("right - Increase volume by 1%"),
					Line::from("ctrl + left - Decrease volume by 5%"),
					Line::from("ctrl + right - Increase volume by 5%"),

					Line::from(""),
					Line::from("Tabs Key Binds").style(Style::default().add_modifier(Modifier::BOLD)).centered(),
					Line::from("a - Add directory"),
					Line::from("d - Remove directory"),
					Line::from("ctrl + arrow keys - Move tab"),

					Line::from(""),
					Line::from("Files Key Binds").style(Style::default().add_modifier(Modifier::BOLD)).centered(),
					Line::from("r - Refresh"),
					Line::from("enter - Play file"),
					Line::from("/ - Play random file"),
					Line::from("x - Set global hotkey"),
					Line::from("z - Remove global hotkey"),
					Line::from("v - Set file ID"),
					Line::from("b - Remove file ID"),
			
					Line::from(""),
					Line::from("Settings Key Binds").style(Style::default().add_modifier(Modifier::BOLD)).centered(),
					Line::from("enter - Set an option"),
					Line::from("delete - Reset an option"),
				]);
			}
			3 => {
				lines.extend(vec![
					Line::from("Waveforms Key Binds").style(Style::default().add_modifier(Modifier::BOLD)).centered(),
					Line::from("a - Add waveform"),
					Line::from("e - Edit waveform"),
					Line::from("r - Rename waveform"),
					Line::from("d - Delete waveform"),
					Line::from("f - Duplicate waveform"),
					Line::from("enter - Play waveform"),
					Line::from("/ - Play random waveform"),
					Line::from("x - Set global hotkey"),
					Line::from("z - Remove global hotkey"),
					Line::from("v - Set waveform ID"),
					Line::from("b - Remove waveform ID"),
				]);
			}
			_ => {}
		}

		let text = Text::from(lines);
		let area = f.area();
		let width = (text.width() as u16) + 4;
		let height = (text.height() as u16) + 4;
		let popup_area = safe_centered_rect(width, height, area);
		Clear.render(popup_area, f.buffer_mut());
		f.render_widget(Paragraph::new(text).block(Block::bordered().padding(Padding::uniform(1)).border_type(BorderType::Rounded)), popup_area);
	}
}

impl PopupHandleKey for HelpPopup {
	fn handle_key(&mut self, event: KeyEvent) -> bool {
		match event.code {
			KeyCode::Char('q')|KeyCode::Esc => {
				defer_exit_popup();
				return true
			},
			KeyCode::Left => self.prev_page(),
			KeyCode::Right => self.next_page(),
			_ => false
		}
	}
}

impl HelpPopup {
	fn next_page(&mut self) -> bool {
		let old = self.page;
		self.page = min(self.page + 1 as u8, self.max_page);
		old != self.page
	}

	fn prev_page(&mut self) -> bool {
		let old = self.page;
		self.page = max(self.page - 1 as u8, 1);
		old != self.page
	}
}