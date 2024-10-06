use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{layout::Rect, style::{Modifier, Style}, text::{Line, Text}, widgets::{Block, BorderType, Clear, Padding, Paragraph, Widget}, Frame};

use crate::constant::{APP_NAME, APP_VERSION};

use super::{exit_popup, PopupHandleKey, PopupRender};

pub struct HelpPopup { }

impl Default for HelpPopup {
	fn default() -> Self {
		Self {}
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
	
			Line::from(""),
			Line::from("Files Key Binds").style(Style::default().add_modifier(Modifier::BOLD)).centered(),
			Line::from("r - Refresh"),
			Line::from("enter - Play file"),
			Line::from("x - Set global key bind"),
			Line::from("z - Remove global key bind"),
		]);
		let area = f.area();
		let width = (text.width() as u16) + 4;
		let height = (text.height() as u16) + 4;
		let popup_area: Rect = Rect {
			x: (area.width - width) / 2,
			y: (area.height - height) / 2,
			width,
			height
		};
		Clear.render(popup_area, f.buffer_mut());
		f.render_widget(Paragraph::new(text).block(Block::bordered().padding(Padding::uniform(1)).border_type(BorderType::Rounded)), popup_area);
	}
}

impl PopupHandleKey for HelpPopup {
	fn handle_key(&mut self, event: KeyEvent) -> bool {
		match event.code {
			KeyCode::Char('q')|KeyCode::Esc => {
				exit_popup();
				return true
			},
			_ => false
		}
	}
}