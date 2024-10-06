use std::{cmp::{max, min}, path::Path};

use crate::{component::popup::{delete_tab::DeleteTabPopup, input::InputPopup, set_popup, PopupComponent}, state::{get_mut_app, AwaitInput}};

use super::{border_style, border_type, BlockHandleKey, BlockRenderArea};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{layout::Rect, style::{Color, Modifier, Style}, text::{Line, Span}, widgets::{Block, Borders, Padding, Paragraph}, Frame};

pub struct TabsBlock {
	title: String,
	id: u8,
	range: (i32, i32),
}

impl Default for TabsBlock {
	fn default() -> Self {
		Self {
			title: "Tabs".to_string(),
			id: 1,
			range: (-1, -1),
		}
	}
}

impl BlockRenderArea for TabsBlock {
	fn render_area(&mut self, f: &mut Frame, area: Rect) {
		let app = get_mut_app();
		let tabs = app.config.tabs.clone();
		let tab_selected = app.tab_selected as usize;
	
		let mut spans: Vec<Span> = vec![];
		for (ii, tab) in tabs.iter().enumerate() {
			let path = Path::new(tab.as_str());
			let basename = path.file_name();
			spans.push(Span::from(basename.unwrap().to_str().unwrap().to_string())
				.style(if ii == tab_selected {
					Style::default().fg(Color::LightGreen).add_modifier(Modifier::REVERSED)
				} else {
					Style::default().fg(Color::Green)
				})
			);
			if ii < tabs.len() - 1 {
				spans.push(Span::from(" | "));
			}
		}
	
		let mut width = area.width as i32 - 4;
		let mut count = 0;
		if self.range.0 == -1 {
			for (ii, span) in spans.iter().enumerate() {
				if ii % 2 == 1 {
					// skip separator
					continue;
				}
				width -= span.width() as i32;
				count += 1;
				if width < 0 {
					break;
				}
			}
			self.range = (0, count - 1);
		} else if app.tab_selected < self.range.0 as usize {
			for (ii, span) in spans.iter().enumerate() {
				if ii % 2 == 1 || ii < app.tab_selected * 2 {
					// skip separator
					continue;
				}
				width -= span.width() as i32;
				count += 1;
				if width < 0 {
					break;
				}
			}
			self.range = (app.tab_selected as i32, app.tab_selected as i32 + count - 1);
		} else if app.tab_selected >= self.range.1 as usize {
			for (ii, span) in spans.iter().rev().enumerate() {
				if ii % 2 == 1 || ii < spans.len() - app.tab_selected * 2 - 1 {
					// skip separator
					continue;
				}
				width -= span.width() as i32;
				count += 1;
				if width < 0 {
					break;
				}
			}
			self.range = (app.tab_selected as i32 - count + 1, app.tab_selected as i32);
		}
		
		let block = Block::default()
			.title(self.title.clone())
			.borders(Borders::ALL)
			.border_type(border_type(self.id))
			.border_style(border_style(self.id));
		let mut length = 0;
		for (ii, span) in spans.iter().enumerate() {
			if ii >= self.range.0 as usize * 2 {
				break;
			}
			length += span.width();
		}
		let paragraph = Paragraph::new(Line::from(spans)).block(block.padding(Padding::horizontal(1))).scroll((0, length as u16));
		f.render_widget(paragraph, area);
	}
}

impl BlockHandleKey for TabsBlock {
	fn handle_key(&self, event: KeyEvent) -> bool {
		match event.code {
			KeyCode::Char('a') => handle_add(),
			KeyCode::Char('d') => handle_remove(),
			KeyCode::Right => handle_move(true, event.modifiers.contains(KeyModifiers::CONTROL)),
			KeyCode::Left => handle_move(false, event.modifiers.contains(KeyModifiers::CONTROL)),
			_ => false
		}
	}
}

fn handle_add() -> bool {
	let app = get_mut_app();
	app.await_input = AwaitInput::AddTab;
	set_popup(PopupComponent::Input(InputPopup::new(std::env::current_dir().unwrap().to_str().unwrap().to_string())));
	true
}

fn handle_remove() -> bool {
	let app = get_mut_app();
	let tab_selected = app.tab_selected;
	if tab_selected < app.config.tabs.len() {
		set_popup(PopupComponent::DeleteTab(DeleteTabPopup::default()));
		return true;
	}
	false
}

fn handle_move(right: bool, modify: bool) -> bool {
	let delta = if right { 1 } else { -1 };
	let app = get_mut_app();
	let tab_selected = app.tab_selected as i32;
	let new_selected = min(app.config.tabs.len() as i32 - 1, max(0, tab_selected + delta));
	if tab_selected != new_selected {
		if modify {
			app.config.tabs.swap(tab_selected as usize, new_selected as usize);
		}
		app.tab_selected = new_selected as usize;
		app.file_selected = 0;
		return true;
	}
	false
}