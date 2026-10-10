use std::{cmp::max, collections::HashSet, vec};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{layout::Rect, style::{Color, Modifier, Style}, text::{Line, Span}, widgets::{Block, Padding, Paragraph}, Frame};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use substring::Substring;

use crate::{client::{AtomicClientState, ClientState, component::{block::BlockNavigation, popup::{PopupComponent, input::{FLAG_NONE, InputPopup}, key_bind::KeyBindPopup}}}, common::socket::ClientToServer};

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
		let stop_key = if config.stop_key.is_empty() {
			"".to_string()
		} else {
			let mut keys = config.stop_key.iter().collect::<Vec<_>>();
			keys.sort();
			keys.iter().map(|key| key.to_string()).collect::<Vec<_>>().join(" + ")
		};
		self.left_right_line("Stop Key".to_string(), stop_key, width as usize, &mut lines);
		self.left_right_line("Loopbacks".to_string(), config.loopbacks.join(","), width as usize, &mut lines);
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
					client_state.config.stop_key = keys.clone();
					client_state.dirty = true;
					client_state.request(ClientToServer::SetStopKey(client_state.config.stop_key.par_iter().cloned().collect()));
				})));
				true
			},
			// Loopbacks
			1 => {
				let loopbacks = client_state.read().config.loopbacks.join(",");
				client_state.clone().read().popup_manager.push(PopupComponent::Input(InputPopup::new(loopbacks, "Loopbacks (Comma-separated)".to_string(), FLAG_NONE, move |value| {
					let mut client_state = client_state.write();
					client_state.config.loopbacks = value.split(",").map(|name| name.to_string()).collect();
					client_state.dirty = true;
					client_state.request(ClientToServer::SetLoopbacks(client_state.config.loopbacks.clone()));
				})));
				true
			},
			// Playlist mode toggle
			2 => {
				let mut client_state = client_state.write();
				client_state.config.playlist_mode = !client_state.config.playlist_mode;
				client_state.dirty = true;
				client_state.request(ClientToServer::SetPlaylistMode(client_state.config.playlist_mode));
				true
			},
			// Fast scan toggle
			3 => {
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
				client_state.config.loopbacks = vec!["@DEFAULT_SINK@".to_string()];
				client_state.dirty = true;
				client_state.request(ClientToServer::SetLoopbacks(client_state.config.loopbacks.clone()));
				true
			},
			2 => {
				client_state.config.playlist_mode = false;
				client_state.dirty = true;
				client_state.request(ClientToServer::SetPlaylistMode(false));
				true
			},
			3 => {
				client_state.config.fast_scan = false;
				client_state.dirty = true;
				true
			},
			_ => false
		}
	}
}