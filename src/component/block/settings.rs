use std::{cmp::max, collections::HashSet, sync::{Mutex, MutexGuard, OnceLock}};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{layout::Rect, style::{Color, Modifier, Style}, text::{Line, Span}, widgets::{Block, Padding, Paragraph}, Frame};
use substring::Substring;

use crate::{component::{block::{files::FilesBlock, waves::WavesBlock, BlockNavigation, BlockSingleton}, popup::{input::{AwaitInput, InputPopup}, key_bind::{KeyBindFor, KeyBindPopup}, set_popup, PopupComponent}}, state::acquire, util::pulseaudio::{loopback, unload_module}};

use super::{loop_index, BlockHandleKey, BlockRenderArea};

pub struct SettingsBlock {
	selected: u8,
	options: u8,
}

impl BlockSingleton for SettingsBlock {
	fn instance() -> MutexGuard<'static, Self> {
		static BLOCK: OnceLock<Mutex<SettingsBlock>> = OnceLock::new();
		BLOCK.get_or_init(|| {
			Mutex::new(SettingsBlock {
				selected: 0,
				options: 5
			})
		}).lock().unwrap()
	}
}

impl BlockRenderArea for SettingsBlock {
	fn render_area(&mut self, f: &mut Frame, area: Rect) {
		let app = acquire();
		let (border_type, border_style) = app.borders(Self::ID);
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
		if app.config.stop_key.is_empty() {
			stop_key = "".to_string();
		} else {
			let mut keys = Vec::from_iter(app.config.stop_key.clone().into_iter());
			keys.sort();
			stop_key = format!("{}", keys.join(" + "));
		}
		self.left_right_line("Stop Key".to_string(), stop_key, width as usize, &mut lines);
		self.left_right_line("Loopback Default".to_string(), app.config.loopback_default.to_string(), width as usize, &mut lines);
		self.left_right_line("Loopback 1".to_string(), app.config.loopback_1.clone(), width as usize, &mut lines);
		self.left_right_line("Loopback 2".to_string(), app.config.loopback_2.clone(), width as usize, &mut lines);
		self.left_right_line("Playlist Mode".to_string(), app.config.playlist_mode.to_string(), width as usize, &mut lines);
		f.render_widget(Paragraph::new(lines).block(block), area);
	}
}

impl BlockHandleKey for SettingsBlock {
	fn handle_key(&mut self, event: KeyEvent) -> bool {
		match event.code {
			KeyCode::Up => self.navigate_settings(-1),
			KeyCode::Down => self.navigate_settings(1),
			KeyCode::Enter => self.handle_enter(),
			KeyCode::Delete => self.handle_delete(),
			_ => false
		}
	}
}

impl BlockNavigation for SettingsBlock {
	const ID: u8 = 3;

	fn navigate_block(&self, dx: i16, _dy: i16) -> u8 {
		if dx < 0 {
			if acquire().waves_opened {
				return WavesBlock::ID;
			}
			return FilesBlock::ID;
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

	fn handle_enter(&mut self) -> bool {
		match self.selected {
			// Stop key
			0 => {
				set_popup(PopupComponent::KeyBind(KeyBindPopup::new(KeyBindFor::Stop, HashSet::new())));
				return true;
			},
			// Loopback default toggle
			1 => {
				let mut app = acquire();
				app.config.loopback_default = !app.config.loopback_default;
				if app.config.loopback_default && app.module_loopback_default.is_empty() {
					app.module_loopback_default = loopback("@DEFAULT_SINK@".to_string()).unwrap_or(String::new());
				} else if !app.config.loopback_default && !app.module_loopback_default.is_empty() {
					app.module_loopback_default = unload_module(&app.module_loopback_default)
						.map_or(app.module_loopback_default.clone(), |_| { String::new() });
				}
				return true;
			},
			// Additional loopback
			2|3 => {
				let await_input;
				if self.selected == 2 {
					await_input = AwaitInput::Loopback1;
				} else {
					await_input = AwaitInput::Loopback2;
				}
				set_popup(PopupComponent::Input(InputPopup::new(String::new(), await_input)));
				return true;
			},
			// Playlist mode toggle
			4 => {
				let mut app = acquire();
				app.config.playlist_mode = !app.config.playlist_mode;
				return true;
			},
			_ => false
		}
	}

	fn handle_delete(&mut self) -> bool {
		let mut app = acquire();
		match self.selected {
			0 => {
				app.config.stop_key.clear();
				app.stopkey.clear();
				return true;
			},
			1 => {
				app.config.loopback_default = true;
				if app.module_loopback_default.is_empty() {
					app.module_loopback_default = loopback("@DEFAULT_SINK@".to_string()).unwrap_or(String::new());
				}
				return true;
			},
			2 => {
				app.config.loopback_1 = String::new();
				if !app.module_loopback_1.is_empty() {
					app.module_loopback_1 = unload_module(&app.module_loopback_1)
						.map_or(app.module_loopback_1.clone(), |_| { String::new() });
				}
				return true;
			},
			3 => {
				app.config.loopback_2 = String::new();
				if !app.module_loopback_2.is_empty() {
					app.module_loopback_2 = unload_module(&app.module_loopback_2)
						.map_or(app.module_loopback_2.clone(), |_| { String::new() });
				}
				return true;
			},
			4 => {
				app.config.playlist_mode = false;
				return true;
			},
			_ => false
		}
	}
}