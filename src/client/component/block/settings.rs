use std::{cmp::max, collections::HashSet, vec};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{layout::Rect, style::{Color, Modifier, Style}, text::{Line, Span}, widgets::{Block, Padding, Paragraph}, Frame};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use substring::Substring;

use crate::{client::{AtomicClientState, ClientState, component::{block::BlockNavigation, popup::{PopupComponent, input::{FLAG_NONE, InputPopup}, key_bind::KeyBindPopup}}}, common::{keyboard::keyboard_to_string, socket::ClientToServer}};

use super::{loop_index, BlockHandleKey, BlockRenderArea};

pub struct SettingsBlock {
	selected: u8,
	options: u8,
}

impl Default for SettingsBlock {
	fn default() -> Self {
		Self {
			selected: 0,
			options: 6
		}
	}
}

impl BlockRenderArea for SettingsBlock {
	fn render_area(&mut self, client_state: &ClientState, f: &mut Frame, area: Rect) {
		let (config, border_type, border_style) = {
			let (border_type, border_style) = client_state.borders(Self::ID);
			(&client_state.config, border_type, border_style)
		};
		let mut block = Block::bordered()
			.border_style(border_style)
			.border_type(border_type)
			.title("Settings");
		let mut width = area.width;

		if area.width > 25 {
			block = block.padding(Padding::horizontal(1));
			width -= 2;
		}

		let mut lines = vec![];
		let stop_key;
		if config.stop_key.is_empty() {
			stop_key = "".to_string();
		} else {
			let mut keys = Vec::from_iter(config.stop_key.clone().into_iter());
			keys.sort();
			stop_key = format!("{}", keys.join(" + "));
		}
		self.left_right_line("Stop Key".to_string(), stop_key, width as usize, &mut lines);
		self.left_right_line("Loopback Default".to_string(), config.loopback_default.to_string(), width as usize, &mut lines);
		self.left_right_line("Loopback 1".to_string(), config.loopback_1.clone(), width as usize, &mut lines);
		self.left_right_line("Loopback 2".to_string(), config.loopback_2.clone(), width as usize, &mut lines);
		self.left_right_line("Playlist Mode".to_string(), config.playlist_mode.to_string(), width as usize, &mut lines);
		self.left_right_line("Fast Scan".to_string(), config.fast_scan.to_string(), width as usize, &mut lines);
		f.render_widget(Paragraph::new(lines).block(block), area);
	}
}

impl BlockHandleKey for SettingsBlock {
	fn handle_key(&mut self, client_state: AtomicClientState, event: KeyEvent) -> bool {
		match event.code {
			KeyCode::Up => self.navigate_settings(-1),
			KeyCode::Down => self.navigate_settings(1),
			KeyCode::Enter => self.handle_enter(client_state),
			KeyCode::Delete => self.handle_delete(client_state),
			_ => false
		}
	}
}

impl BlockNavigation for SettingsBlock {
	const ID: u8 = 3;

	fn navigate_block(&self, client_state: AtomicClientState, dx: i16, _dy: i16) -> u8 {
		if dx < 0 {
			return client_state.read().main_opened.id(Self::ID);
		}
		Self::ID
	}
}

impl SettingsBlock {
	fn left_right_line(&self, left: String, mut right: String, width: usize, lines: &mut Vec<Line>) {
		right = " ".to_owned() + right.as_str();
		let mid_span;
		if left.len() + right.len() >= width as usize {
			mid_span = Span::from("");
			right = right.substring(0, right.len() - 3).to_string() + "...";
		} else {
			mid_span = Span::from(vec![" "; max(0, width as i32 - left.len() as i32 - right.len() as i32 - 2) as usize].join(""));
		}
		let left_style;
		if lines.len() == self.selected as usize {
			left_style = Style::default().fg(Color::LightYellow).add_modifier(Modifier::REVERSED);
		} else {
			left_style = Style::default().fg(Color::LightYellow);
		}
		let left_span = Span::from(left).style(left_style);
		let right_span = Span::from(right).style(Style::default().fg(Color::Yellow));

		lines.push(Line::from(vec![left_span, mid_span, right_span]));
	}

	fn navigate_settings(&mut self, dy: i16) -> bool {
		let new_selected = loop_index(self.selected as usize, dy as i32, self.options as usize) as usize;
		let new_selected = new_selected as u8;
		if new_selected != self.selected {
			self.selected = new_selected;
			return true;
		}
		false
	}

	fn handle_enter(&mut self, client_state: AtomicClientState) -> bool {
		match self.selected {
			// Stop key
			0 => {
				client_state.clone().read().popup_manager.push(PopupComponent::KeyBind(KeyBindPopup::new(HashSet::new(), move |keys| {
					let mut client_state = client_state.write();
					client_state.config.stop_key = keys.par_iter().map(|key| keyboard_to_string(*key)).collect();
					client_state.dirty = true;
					client_state.request(ClientToServer::SetStopKey(client_state.config.stop_key.par_iter().cloned().collect()));
				})));
				true
			},
			// Loopback default toggle
			1 => {
				let mut client_state = client_state.write();
				client_state.config.loopback_default = !client_state.config.loopback_default;
				client_state.dirty = true;
				client_state.request(ClientToServer::SetLoopback(1, if client_state.config.loopback_default { "@DEFAULT_SINK@" } else { "" }.to_string()));
				true
			},
			// Additional loopback
			2|3 => {
				let selected = self.selected;
				client_state.clone().read().popup_manager.push(PopupComponent::Input(InputPopup::new(String::new(), if self.selected == 2 { "Loopback 1" } else { "Loopback 2" }.to_string(), FLAG_NONE, move |value| {
					let mut client_state = client_state.write();
					let loopback = value.to_string();
					if selected == 2 {
						client_state.config.loopback_1 = loopback.clone();
						client_state.dirty = true;
						client_state.request(ClientToServer::SetLoopback(2, loopback.clone()));
					} else {
						client_state.config.loopback_2 = loopback.clone();
						client_state.dirty = true;
						client_state.request(ClientToServer::SetLoopback(3, loopback.clone()));
					}
				})));
				true
			},
			// Playlist mode toggle
			4 => {
				let mut client_state = client_state.write();
				client_state.config.playlist_mode = !client_state.config.playlist_mode;
				client_state.dirty = true;
				client_state.request(ClientToServer::SetPlaylistMode(client_state.config.playlist_mode));
				true
			},
			// Fast scan toggle
			5 => {
				let mut client_state = client_state.write();
				client_state.config.fast_scan = !client_state.config.fast_scan;
				client_state.dirty = true;
				true
			},
			_ => false
		}
	}

	fn handle_delete(&mut self, client_state: AtomicClientState) -> bool {
		let mut client_state = client_state.write();
		match self.selected {
			0 => {
				client_state.config.stop_key.clear();
				client_state.dirty = true;
				client_state.request(ClientToServer::SetStopKey(vec![]));
				true
			},
			1 => {
				client_state.config.loopback_default = true;
				client_state.dirty = true;
				client_state.request(ClientToServer::SetLoopback(1, "@DEFAULT_SINK@".to_string()));
				true
			},
			2 => {
				client_state.config.loopback_1 = String::new();
				client_state.dirty = true;
				client_state.request(ClientToServer::SetLoopback(2, "".to_string()));
				true
			},
			3 => {
				client_state.config.loopback_2 = String::new();
				client_state.dirty = true;
				client_state.request(ClientToServer::SetLoopback(3, "".to_string()));
				true
			},
			4 => {
				client_state.config.playlist_mode = false;
				client_state.dirty = true;
				client_state.request(ClientToServer::SetPlaylistMode(false));
				true
			},
			5 => {
				client_state.config.fast_scan = false;
				client_state.dirty = true;
				true
			},
			_ => false
		}
	}
}