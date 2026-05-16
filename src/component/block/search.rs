use std::{sync::{Mutex, MutexGuard, OnceLock}};

use crate::{component::block::{BlockNavigation, BlockSingleton, info::InfoBlock, results::ResultsBlock}, state::{MainOpened, acquire}};

use super::{BlockHandleKey, BlockRenderArea};

use crossterm::event::{Event, KeyCode, KeyEvent};
use ratatui::{Frame, layout::Rect, style::Color, widgets::{Block, Borders, Padding, Paragraph}};
use tui_input::{Input, backend::crossterm::EventHandler};

pub struct SearchBlock {
	input: Input,
}

impl BlockSingleton for SearchBlock {
	fn instance() -> MutexGuard<'static, Self> {
		static BLOCK: OnceLock<Mutex<SearchBlock>> = OnceLock::new();
		BLOCK.get_or_init(|| {
			Mutex::new(SearchBlock {
				input: Input::default(),
			})
		}).lock().unwrap()
	}
}

impl BlockRenderArea for SearchBlock {
	fn render_area(&mut self, f: &mut Frame, area: Rect) {
		let app = acquire();
	
		let width = area.width as usize - 4;
		let (border_type, border_style) = app.borders(Self::ID);
		let block = Block::default()
			.title("Search")
			.borders(Borders::ALL)
			.border_type(border_type)
			.border_style(border_style.fg(if app.block_selected == Self::ID { Color::LightGreen } else { Color::Green }));
		let scroll = self.input.visual_scroll(width as usize - 1);
		let paragraph = Paragraph::new(self.input.value()).block(block.padding(Padding::horizontal(1))).scroll((0, scroll as u16));
		f.render_widget(paragraph, area);
		f.set_cursor_position((
			area.x + ((self.input.visual_cursor()).max(scroll) - scroll) as u16 + 2,
			area.y + 1
		));
	}
}

impl BlockHandleKey for SearchBlock {
	fn handle_key(&mut self, event: KeyEvent) -> bool {
		match event.code {
			KeyCode::Esc => {
				acquire().main_opened = MainOpened::File;
				true
			},
			KeyCode::Enter => {
				ResultsBlock::instance().search(self.input.value());
				true
			},
			_ => {
				self.input.handle_event(&Event::Key(event));
				true
			}
		}
	}
}

impl BlockNavigation for SearchBlock {
	const ID: u8 = 8;

	fn navigate_block(&self, _dx: i16, dy: i16) -> u8 {
		if dy > 0 {
			return acquire().main_opened.id(Self::ID);
		} else if dy < 0 {
			return InfoBlock::ID;
		}
		return Self::ID;
	}
}