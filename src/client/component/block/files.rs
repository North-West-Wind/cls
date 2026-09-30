use std::{cmp::{max, min}, i32, path::Path, vec};

use crate::{client::{AtomicClientState, ClientState, Scanning, component::{block::{BlockNavigation, settings::SettingsBlock, tabs::TabsBlock}, popup::{PopupComponent, input::{FLAG_INT, InputPopup}, key_bind::KeyBindPopup}}, tab::scan}, common::{keyboard::{keyboard_to_string, string_to_keyboard}, socket::ClientToServer}};

use super::{loop_index, BlockHandleKey, BlockRenderArea};

use crossterm::event::KeyCode;
use rand::Rng;
use ratatui::{layout::Rect, style::{Color, Modifier, Style}, text::{Line, Span}, widgets::{Block, Borders, Padding, Paragraph, Wrap}, Frame};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use substring::Substring;

pub struct FilesBlock {
	range: (i32, i32),
	height: u16,
}

impl Default for FilesBlock {
	fn default() -> Self {
		Self {
			range: (-1, -1),
			height: 0
		}
	}
}

impl BlockRenderArea for FilesBlock {
	fn render_area(&mut self, client_state: &ClientState, f: &mut Frame, area: Rect) {
		let (border_type, border_style) = client_state.borders(Self::ID);
		let block = Block::default()
			.title("Files")
			.borders(Borders::ALL)
			.border_type(border_type)
			.border_style(border_style)
			.padding(Padding::new(2, 2, 1, 1));
	
		if self.range.0 == -1 || self.height != area.height {
			self.range = (0, area.height as i32 - 5);
			self.height = area.height;
		}
		let paragraph: Paragraph;
		if client_state.file_tabs.len() == 0 {
			paragraph = Paragraph::new("Add a tab to get started :>").wrap(Wrap { trim: false });
		} else {
			let (_, files) = &client_state.file_tabs[client_state.selected_tab];
			paragraph = if files.len() == 0 {
					if client_state.scanning == Scanning::All {
						Paragraph::new("Performing initial scan...").wrap(Wrap { trim: false })
					} else if client_state.scanning == Scanning::One(client_state.selected_tab) {
						Paragraph::new("Scanning this directory...\nComeback later :>").wrap(Wrap { trim: false })
					} else {
						Paragraph::new("There are no playable files in this directory :<").wrap(Wrap { trim: false })
					}
				} else {
					let lines = files.iter().enumerate().map(|(ii, (file, info))| {
						let mut spans = vec![];
						spans.push(info.base.id.map_or(Span::from(" "), |_| { Span::from("I").style(Style::default().fg(Color::LightYellow).add_modifier(Modifier::REVERSED)) }));
						if info.base.keys.is_empty() {
							spans.push(Span::from(" "));
						} else {
							spans.push(Span::from("K").style(Style::default().fg(Color::LightGreen).add_modifier(Modifier::REVERSED)));
						}
						spans.push(Span::from(" "));
						let style = if info.duration.is_empty() {
							let tmp = Style::default().fg(Color::Red);
							if client_state.selected_file == ii {
								tmp.add_modifier(Modifier::REVERSED)
							} else {
								tmp
							}
						} else if client_state.selected_file == ii {
							Style::default().fg(Color::LightBlue)
							.add_modifier(Modifier::REVERSED)
						} else {
							Style::default().fg(Color::Cyan)
						};
						let extra: usize = spans.par_iter().map(|span| { span.width() }).sum();
						if file.len() + info.duration.len() + extra as usize > area.width as usize - 6 {
							spans.push(Span::from(file.substring(0, max(0, area.width as i32 - 10 - extra as i32 - info.duration.len() as i32) as usize)).style(style));
							spans.push(Span::from("... ".to_owned() + &info.duration).style(style));
						} else {
							spans.push(Span::from(file.clone()).style(style));
							spans.push(Span::from(vec![" "; max(0, area.width as i32 - 6 - extra as i32 - file.len() as i32 - info.duration.len() as i32) as usize].join("")).style(style));
							spans.push(Span::from(info.duration.clone()).style(style));
						}
						Line::from(spans)
					}).collect::<Vec<_>>();
					if client_state.selected_file < self.range.0 as usize {
						self.range = (client_state.selected_file as i32, client_state.selected_file as i32 + area.height as i32 - 5);
					} else if client_state.selected_file > self.range.1 as usize {
						self.range = (client_state.selected_file as i32 - area.height as i32 + 5, client_state.selected_file as i32);
					}
					Paragraph::new(lines).scroll((self.range.0 as u16, 0))
				};
		}
		f.render_widget(paragraph.block(block), area);
	}
}

impl BlockHandleKey for FilesBlock {
	fn handle_key(&mut self, client_state: AtomicClientState, event: crossterm::event::KeyEvent) -> bool {
		match event.code {
			KeyCode::Char('r') => self.reload_tab(client_state),
			KeyCode::Up => self.navigate_file(client_state, -1),
			KeyCode::Down => self.navigate_file(client_state, 1),
			KeyCode::Enter => self.play_file(client_state, false),
			KeyCode::Char('/') => self.play_file(client_state, true),
			KeyCode::Char('x') => self.set_global_key_bind(client_state),
			KeyCode::Char('z') => self.unset_global_key_bind(client_state),
			KeyCode::Char('v') => self.set_file_id(client_state),
			KeyCode::Char('b') => self.unset_file_id(client_state),
			KeyCode::PageUp => self.navigate_file(client_state, -(self.range.1 - self.range.0 + 1)),
			KeyCode::PageDown => self.navigate_file(client_state, self.range.1 - self.range.0 + 1),
			KeyCode::Home => self.navigate_file(client_state, -i32::MAX),
			KeyCode::End => self.navigate_file(client_state, i32::MAX),
			_ => false,
		}
	}
}

impl BlockNavigation for FilesBlock {
	const ID: u8 = 2;

	fn navigate_block(&self, client_state: AtomicClientState, dx: i16, dy: i16) -> u8 {
		if dy < 0 {
			return TabsBlock::ID;
		}
		if dx > 0 && client_state.read().settings_opened {
			return SettingsBlock::ID;
		}
		Self::ID
	}
}

impl FilesBlock {
	fn play_file(&self, client_state: AtomicClientState, random: bool) -> bool {
		let client_state = client_state.read();
		if client_state.selected_tab >= client_state.file_tabs.len() {
			return false;
		}
		let (tab, files) = &client_state.file_tabs[client_state.selected_tab];
		let index;
		if random {
			index = rand::thread_rng().gen_range(0..files.len());
		} else {
			if client_state.selected_file >= files.len() {
				return false;
			}
			index = client_state.selected_file;
		}
		if let Some((name, _)) = files.get_index(index) {
			let path = Path::new(tab).join(name).to_str().unwrap().to_string();
			client_state.request(ClientToServer::PlayPath(path))
		} else {
			false
		}
	}

	fn navigate_file(&mut self, client_state: AtomicClientState, dy: i32) -> bool {
		let mut client_state = client_state.write();
		if client_state.file_tabs.is_empty() {
			return false;
		}
		let (_, files) = &client_state.file_tabs[client_state.selected_tab];
		let files = files.len();
		let new_selected = if dy.abs() > 1 {
			min(files as i32 - 1, max(0, client_state.selected_file as i32 + dy)) as usize
		} else {
			loop_index(client_state.selected_file, dy, files)
		};
		if new_selected != client_state.selected_file {
			client_state.selected_file = new_selected;
			return true;
		}
		false
	}

	fn reload_tab(&self, atomic_client_state: AtomicClientState) -> bool {
		let client_state = atomic_client_state.read();
		if client_state.selected_tab < client_state.file_tabs.len() {
			scan(atomic_client_state.clone(), Scanning::One(client_state.selected_tab));
			return true;
		}
		false
	}

	fn set_global_key_bind(&self, client_state: AtomicClientState) -> bool {
		let (init, popup_manager) = {
			let client_state = client_state.read();
			(
				client_state.file_tabs[client_state.selected_tab].1[client_state.selected_file].base.keys.par_iter().filter_map(|key| string_to_keyboard(key)).collect(),
				client_state.popup_manager.clone()
			)
		};
		popup_manager.push(PopupComponent::KeyBind(KeyBindPopup::new(init, move |keys| {
			let mut client_state = client_state.write();
			let (selected_tab, selected_file) = (client_state.selected_tab, client_state.selected_file);
			client_state.file_tabs[selected_tab].1[selected_file].base.keys = keys.iter().map(|key| keyboard_to_string(*key)).collect();
		})));
		return true;
	}

	fn unset_global_key_bind(&self, client_state: AtomicClientState) -> bool {
		let mut client_state = client_state.write();
		let (selected_tab, selected_file) = (client_state.selected_tab, client_state.selected_file);
		client_state.file_tabs[selected_tab].1[selected_file].base.keys.clear();
		true
	}

	fn set_file_id(&self, client_state: AtomicClientState) -> bool {
		let (name, init, popup_manager) = {
			let client_state = client_state.read();
			if let Some((name, init)) = client_state.file_tabs[client_state.selected_tab].1.get_index(client_state.selected_file) {
				(
					name.clone(),
					if let Some(id) = init.base.id { id.to_string() } else { String::new() },
					client_state.popup_manager.clone()
				)
			} else {
				(String::new(), String::new(), client_state.popup_manager.clone())
			}
		};
		popup_manager.push(PopupComponent::Input(InputPopup::new(init, "File ID".to_string(), FLAG_INT, move |value| {
			let Ok(id) = u32::from_str_radix(value, 10) else { return; };
			let mut client_state = client_state.write();
			let existing = client_state.file_tabs.iter_mut().find_map(|(_, files)| {
				files.iter().find_map(|(name, info)| {
					if info.base.id == Some(id) {
						Some(name.clone())
					} else {
						None
					}
				})
			});
			if let Some(existing) = existing {
				if existing != name {
					client_state.error = "File ID must be unique".to_string();
				}
				return;
			}

			let (selected_tab, selected_file) = (client_state.selected_tab, client_state.selected_file);
			client_state.file_tabs[selected_tab].1[selected_file].base.id = Some(id);
		})));
		return true;
	}

	fn unset_file_id(&self, client_state: AtomicClientState) -> bool {
		let mut client_state = client_state.write();
		let (selected_tab, selected_file) = (client_state.selected_tab, client_state.selected_file);
		client_state.file_tabs[selected_tab].1[selected_file].base.id = None;
		true
	}
}

