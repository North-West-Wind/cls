use std::cmp::max;

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{style::{Modifier, Style}, text::{Line, Text}, widgets::{Block, BorderType, Clear, Padding, Paragraph, Widget}, Frame};

use crate::constant::{APP_NAME, APP_VERSION};

use super::{exit_popup, safe_centered_rect, PopupHandleKey, PopupRender};

pub struct HelpPopup {
	scroll: (i32, i32)
}

impl Default for HelpPopup {
	fn default() -> Self {
		Self {
			scroll: (0, 0)
		}
	}
}

impl PopupRender for HelpPopup {
	fn render(&self, f: &mut Frame) {
		let appname = APP_NAME;
		let text = Text::from(vec![
			Line::from(format!("{appname} - Command Line Soundboard")).style(Style::default().add_modifier(Modifier::BOLD)).centered(),
			Line::from(APP_VERSION).style(Style::default().add_modifier(Modifier::BOLD)).centered(),
			Line::from(""),
	
			Line::from("Root Key Binds").style(Style::default().add_modifier(Modifier::BOLD)).centered(),
			Line::from("? - Help"),
			Line::from("q / esc - Escape / Quit"),
			Line::from("arrow keys - Navigate"),
			Line::from("enter - Select block"),
			Line::from("c - Toggle settings menu"),
			Line::from("s - Save configuration"),
	
			Line::from(""),
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
			Line::from("w - Play random file"),
			Line::from("x - Set global hotkey"),
			Line::from("z - Remove global hotkey"),
	
			Line::from(""),
			Line::from("Settings Key Binds").style(Style::default().add_modifier(Modifier::BOLD)).centered(),
			Line::from("enter - Edit an option"),
			Line::from("delete - Reset an option"),
		]);
		let area = f.area();
		let width = (text.width() as u16) + 4;
		let height = (text.height() as u16) + 4;
		let popup_area = safe_centered_rect(width, height, area);
		Clear.render(popup_area, f.buffer_mut());
		f.render_widget(Paragraph::new(text).block(Block::bordered().padding(Padding::uniform(1)).border_type(BorderType::Rounded)).scroll((max(0, self.scroll.1) as u16, max(0, self.scroll.0) as u16)), popup_area);
	}
}

impl PopupHandleKey for HelpPopup {
	fn handle_key(&mut self, event: KeyEvent) -> bool {
		match event.code {
			KeyCode::Char('q')|KeyCode::Esc => {
				exit_popup();
				return true
			},
			KeyCode::Up => scroll(self, 0, -1),
			KeyCode::Down => scroll(self, 0, 1),
			KeyCode::Left => scroll(self, -1, 0),
			KeyCode::Right => scroll(self, 1, 0),
			_ => false
		}
	}
}

fn scroll(popup: &mut HelpPopup, dx: i32, dy: i32) -> bool {
	popup.scroll.0 += dx;
	popup.scroll.1 += dy;
	true
}