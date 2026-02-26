use std::{path::Path, sync::{Mutex, MutexGuard, OnceLock}};

use crate::{component::{block::{BlockNavigation, BlockSingleton, files::FilesBlock, info::InfoBlock}, popup::{PopupComponent, confirm::{ConfirmAction, ConfirmPopup}, input::{AwaitInput, InputPopup}, set_popup}}, state::acquire};

use super::{loop_index, BlockHandleKey, BlockRenderArea};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{layout::Rect, style::{Color, Modifier, Style}, text::{Line, Span}, widgets::{Block, Borders, Padding, Paragraph}, Frame};

pub struct TabsBlock {
	offset: usize,
	pub selected: usize,
}

impl BlockSingleton for TabsBlock {
	fn instance() -> MutexGuard<'static, Self> {
		static BLOCK: OnceLock<Mutex<TabsBlock>> = OnceLock::new();
		BLOCK.get_or_init(|| {
			Mutex::new(TabsBlock {
				offset: 0,
				selected: 0
			})
		}).lock().unwrap()
	}
}

impl BlockRenderArea for TabsBlock {
	fn render_area(&mut self, f: &mut Frame, area: Rect) {
		let app = acquire();
		let tabs = app.config.tabs.clone();
	
		let mut spans: Vec<Span> = vec![];
		let mut cursor = 0;
		for (ii, tab) in tabs.iter().enumerate() {
			let path = Path::new(tab.as_str());
			let basename = path.file_name();
			let str = basename.unwrap().to_str().unwrap().to_string();
			spans.push(Span::from(str)
				.style(if ii == self.selected {
					cursor = spans.len();
					Style::default().fg(Color::LightGreen).add_modifier(Modifier::REVERSED)
				} else {
					Style::default().fg(Color::Green)
				})
			);
			if ii < tabs.len() - 1 {
				spans.push(Span::from(" | "));

			}
		}
	
		let width = area.width as usize - 4;
		let mut wanted_range = (0, 0);
		for (ii, span) in spans.iter().enumerate() {
			wanted_range.0 = wanted_range.1;
			wanted_range.1 += span.width();
			if ii == cursor {
				break;
			}
		}
		if self.offset > wanted_range.0 {
			self.offset = wanted_range.0;	
		} else if self.offset + width < wanted_range.1 {
			self.offset = wanted_range.1 - width;
		}
		
		let (border_type, border_style) = app.borders(Self::ID);
		let block = Block::default()
			.title("Tabs")
			.borders(Borders::ALL)
			.border_type(border_type)
			.border_style(border_style);
		let paragraph = Paragraph::new(Line::from(spans)).block(block.padding(Padding::horizontal(1))).scroll((0, self.offset as u16));
		f.render_widget(paragraph, area);
	}
}

impl BlockHandleKey for TabsBlock {
	fn handle_key(&mut self, event: KeyEvent) -> bool {
		match event.code {
			KeyCode::Char('a') => self.handle_add(),
			KeyCode::Char('d') => self.handle_remove(),
			KeyCode::Right => self.handle_move(true, event.modifiers.contains(KeyModifiers::CONTROL)),
			KeyCode::Left => self.handle_move(false, event.modifiers.contains(KeyModifiers::CONTROL)),
			_ => false
		}
	}
}

impl BlockNavigation for TabsBlock {
	const ID: u8 = 1;

	fn navigate_block(&self, _dx: i16, dy: i16) -> u8 {
		if dy > 0 {
			return acquire().main_opened.id(Self::ID);
		} else if dy < 0 {
			return InfoBlock::ID;
		}
		return Self::ID;
	}
}

impl TabsBlock {
	fn handle_remove(&self) -> bool {
		if self.selected < acquire().config.tabs.len() {
			set_popup(PopupComponent::Confirm(ConfirmPopup::new(ConfirmAction::DeleteTab)));
			return true;
		}
		false
	}

	fn handle_move(&mut self, right: bool, modify: bool) -> bool {
		let delta = if right { 1 } else { -1 };
		let mut app = acquire();
		let new_selected = loop_index(self.selected, delta, app.config.tabs.len());
		if self.selected != new_selected {
			if modify {
				app.config.tabs.swap(self.selected, new_selected as usize);
			}
			self.selected = new_selected as usize;
			FilesBlock::instance().selected = 0;
			return true;
		}
		false
	}

	fn handle_add(&self) -> bool {
		set_popup(PopupComponent::Input(InputPopup::new(std::env::current_dir().unwrap().to_str().unwrap().to_string(), AwaitInput::AddTab)));
		true
	}
}