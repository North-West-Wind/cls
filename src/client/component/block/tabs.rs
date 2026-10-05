use std::{path::Path, vec};

use crate::client::{AtomicClientState, ClientState, Scanning, component::{block::{BlockNavigation, info::InfoBlock}, popup::{PopupComponent, confirm::ConfirmPopup, input::{FLAG_DIR, InputPopup}}}, tab::scan};

use super::{loop_index, BlockHandleKey, BlockRenderArea};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use indexmap::IndexMap;
use normpath::PathExt;
use ratatui::{layout::Rect, style::{Color, Modifier, Style}, text::{Line, Span}, widgets::{Block, Borders, Padding, Paragraph}, Frame};

#[derive(Default)]
pub struct TabsBlock {
	offset: usize,
}

impl BlockRenderArea for TabsBlock {
	fn render_area(&mut self, client_state: &ClientState, f: &mut Frame, area: Rect) {
		let spans = client_state.file_tabs.keys().enumerate().map(|(ii, tab)| {
			let path = Path::new(tab);
			let basename = path.file_name();
			let str = basename.unwrap().to_str().unwrap().to_string();
			let span = Span::from(str).style(if ii == client_state.selected_tab {
				Style::default().fg(Color::LightGreen).add_modifier(Modifier::REVERSED)
			} else {
				Style::default().fg(Color::Green)
			});
			vec![span]
		}).collect::<Vec<_>>().join(&Span::from(" | "));
	
		let width = area.width as usize - 4;
		let mut wanted_range = (0, 0);
		for (ii, span) in spans.iter().enumerate() {
			wanted_range.0 = wanted_range.1;
			wanted_range.1 += span.width();
			if ii == client_state.selected_tab * 2 {
				break;
			}
		}
		if self.offset > wanted_range.0 {
			self.offset = wanted_range.0;	
		} else if self.offset + width < wanted_range.1 {
			self.offset = wanted_range.1 - width;
		}
		
		let (border_type, border_style) = client_state.borders(Self::ID);
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
	fn handle_key(&mut self, client_state: AtomicClientState, event: KeyEvent) -> bool {
		match event.code {
			KeyCode::Char('a') => self.handle_add(client_state),
			KeyCode::Char('d') => self.handle_remove(client_state),
			KeyCode::Right => self.handle_move(client_state, true, event.modifiers.contains(KeyModifiers::CONTROL)),
			KeyCode::Left => self.handle_move(client_state, false, event.modifiers.contains(KeyModifiers::CONTROL)),
			_ => false
		}
	}
}

impl BlockNavigation for TabsBlock {
	const ID: u8 = 1;

	fn navigate_block(&self, client_state: AtomicClientState, _dx: i16, dy: i16) -> u8 {
		if dy > 0 {
			return client_state.read().main_opened.id(Self::ID);
		} else if dy < 0 {
			return InfoBlock::ID;
		}
		return Self::ID;
	}
}

impl TabsBlock {
	fn handle_remove(&self, client_state: AtomicClientState) -> bool {
		let (selected, length, popup_manager) = {
			let client_state = client_state.read();
			(client_state.selected_tab, client_state.file_tabs.len(), client_state.popup_manager.clone())
		};
		if selected < length {
			popup_manager.push(PopupComponent::Confirm(ConfirmPopup::new("Delete tab?", "delete", move || {
				let mut client_state = client_state.write();
				let selected = client_state.selected_tab;
				client_state.file_tabs.shift_remove_index(selected);
				let length = client_state.file_tabs.len();
				if selected >= length && length != 0 {
					client_state.selected_tab = length - 1;
				}
			})));
			return true;
		}
		false
	}

	fn handle_move(&mut self, client_state: AtomicClientState, right: bool, modify: bool) -> bool {
		let mut client_state = client_state.write();
		let delta = if right { 1 } else { -1 };
		let selected = client_state.selected_tab;
		let new_selected = loop_index(selected, delta, client_state.file_tabs.len());
		if selected != new_selected {
			if modify {
				client_state.file_tabs.swap_indices(selected, new_selected);
				client_state.dirty = true;
			}
			client_state.selected_tab = new_selected as usize;
			client_state.selected_file = 0;
			return true;
		}
		false
	}

	fn handle_add(&self, client_state: AtomicClientState) -> bool {
		client_state.clone().read().popup_manager.push(PopupComponent::Input(InputPopup::new(std::env::current_dir().unwrap().to_str().unwrap().to_string(), "Add Directory as Tab".to_string(), FLAG_DIR, move |value| {
			let Ok(norm) = Path::new(value).normalize() else { return; };
			let index = {
				let mut client_state = client_state.write();
				client_state.file_tabs.insert(norm.clone().into_os_string().into_string().unwrap(), IndexMap::new());
				client_state.selected_tab = client_state.file_tabs.len() - 1;
				client_state.dirty = true;
				client_state.selected_tab
			};
			scan(client_state.clone(), Scanning::One(index));
		})));
		true
	}
}