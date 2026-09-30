use std::{path::Path, vec};

use crossterm::event::KeyCode;
use rand::Rng;
use ratatui::{Frame, layout::Rect, style::{Color, Modifier, Style}, text::{Line, Span}, widgets::{Block, Borders, Padding, Paragraph}};
use rayon::iter::{IndexedParallelIterator, IntoParallelRefIterator, ParallelIterator};
use substring::Substring;

use crate::{client::{AtomicClientState, ClientState, SearchResult, SearchState, component::block::{BlockHandleKey, BlockNavigation, BlockRenderArea, loop_index, search::SearchBlock, settings::SettingsBlock}}, common::socket::ClientToServer};

pub struct ResultsBlock {
	range: (i32, i32),
	height: u16,
}

impl Default for ResultsBlock {
	fn default() -> Self {
		Self {
			range: (-1, -1),
			height: 0,
		}
	}
}

impl BlockRenderArea for ResultsBlock {
	fn render_area(&mut self, client_state: &ClientState, f: &mut Frame, area: Rect) {
		let (border_type, border_style) = client_state.borders(Self::ID);
		let block = Block::default()
			.title("Results")
			.borders(Borders::ALL)
			.border_type(border_type)
			.border_style(border_style.fg(if client_state.selected_block == Self::ID { Color::LightGreen } else { Color::Green }))
			.padding(Padding::new(2, 2, 1, 1));

		if self.range.0 == -1 || self.height != area.height {
			self.range = (0, area.height as i32 - 5);
			self.height = area.height;
		}

		use SearchState::*;
		let paragraph = match client_state.search_state {
			Initial => Paragraph::new("Enter a query to search"),
			Searching => Paragraph::new("Searching..."),
			Finish => {
				if client_state.search_results.len() == 0 {
					Paragraph::new("Result is empty :<")
				} else {
					use SearchResult::*;
					let lines = client_state.search_results.par_iter().enumerate().map(| (ii, (_, result_type))| {
						let mut spans = vec![];
						let (has_id, has_keys, main, right, style) = match result_type {
							File(result) => {
								let (_, name, info) = (&result.parent, &result.name, &result.info);
								let has_id = info.base.id.is_some();
								let has_keys = !info.base.keys.is_empty();
								let style = if client_state.selected_result == ii {
									Style::default().add_modifier(Modifier::REVERSED)
								} else {
									Style::default()
								};
								(has_id, has_keys, name, &info.duration, style)
							},
							Wave(result) => {
								let style = if client_state.selected_result == ii {
									Style::default().fg(Color::LightBlue).add_modifier(Modifier::REVERSED)
								} else {
									Style::default().fg(Color::Cyan)
								};
								(result.has_id, result.has_keys, &result.main, &result.sub, style)
							},
							Dialog(result) => {
								let style = if client_state.selected_result == ii {
									Style::default().fg(Color::LightYellow).add_modifier(Modifier::REVERSED)
								} else {
									Style::default().fg(Color::Yellow)
								};
								(result.has_id, result.has_keys, &result.main, &result.sub, style)
							}
						};
						// Construct the line
						if has_id {
							spans.push(Span::from("I").style(Style::default().fg(Color::LightYellow).add_modifier(Modifier::REVERSED)));
						} else {
							spans.push(Span::from(" "));
						}
						if has_keys {
							spans.push(Span::from("K").style(Style::default().fg(Color::LightGreen).add_modifier(Modifier::REVERSED)));
						} else {
							spans.push(Span::from(" "));
						}
						spans.push(Span::from(" "));
						let extra: usize = spans.par_iter().map(|span| span.width()).sum();
						if main.len() + right.len() + extra as usize > area.width as usize - 6 {
							spans.push(Span::from(main.substring(0, 0.max(area.width as i32 - 10 - extra as i32 - right.len() as i32) as usize)).style(style));
							spans.push(Span::from("... ".to_owned() + &right).style(style));
						} else {
							spans.push(Span::from(main.clone()).style(style));
							spans.push(Span::from(vec![" "; 0.max(area.width as i32 - 6 - extra as i32 - main.len() as i32 - right.len() as i32) as usize].join("")).style(style));
							spans.push(Span::from(right.clone()).style(style));
						}
						Line::from(spans)
					}).collect::<Vec<_>>();
					Paragraph::new(lines)
				}
			}
		};
		if client_state.selected_result < self.range.0 as usize {
			self.range = (client_state.selected_result as i32, client_state.selected_result as i32 + area.height as i32 - 5);
		} else if client_state.selected_result > self.range.1 as usize {
			self.range = (client_state.selected_result as i32 - area.height as i32 + 5, client_state.selected_result as i32);
		}
		f.render_widget(paragraph.scroll((self.range.0 as u16, 0)).block(block), area);
	}
}

impl BlockHandleKey for ResultsBlock {
	fn handle_key(&mut self, client_state: AtomicClientState, event: crossterm::event::KeyEvent) -> bool {
		match event.code {
			KeyCode::Up => self.navigate_file(client_state, -1),
			KeyCode::Down => self.navigate_file(client_state, 1),
			KeyCode::Enter => self.play(client_state, false),
			KeyCode::Char('/') => self.play(client_state, true),
			KeyCode::PageUp => self.navigate_file(client_state, -(self.range.1 - self.range.0 + 1)),
			KeyCode::PageDown => self.navigate_file(client_state, self.range.1 - self.range.0 + 1),
			KeyCode::Home => self.navigate_file(client_state, -i32::MAX),
			KeyCode::End => self.navigate_file(client_state, i32::MAX),
			_ => false,
		}
	}
}

impl BlockNavigation for ResultsBlock {
	const ID: u8 = 9;

	fn navigate_block(&self, client_state: AtomicClientState, dx: i16, dy: i16) -> u8 {
		if dy < 0 {
			return SearchBlock::ID;
		}
		if dx > 0 && client_state.read().settings_opened {
			return SettingsBlock::ID;
		}
		Self::ID
	}
}

impl ResultsBlock {
	fn play(&self, client_state: AtomicClientState, random: bool) -> bool {
		let client_state = client_state.read();
		if client_state.search_results.len() == 0 {
			return false;
		}
		let index;
		if random {
			index = rand::thread_rng().gen_range(0..client_state.search_results.len());
		} else {
			if client_state.selected_result >= client_state.search_results.len() {
				return false;
			}
			index = client_state.selected_result;
		}
		use SearchResult::*;
		match &client_state.search_results[index].1 {
			File(result) => {
				let path = Path::new(&result.parent).join(&result.name).to_str().unwrap().to_string();
				client_state.request(ClientToServer::PlayPath(path))
			},
			Wave(result) => client_state.request(ClientToServer::PlayWave(result.uid)),
			Dialog(result) => client_state.request(ClientToServer::PlayDialog(result.uid))
		}
	}

	fn navigate_file(&mut self, client_state: AtomicClientState, dy: i32) -> bool {
		let mut client_state = client_state.write();
		let files = client_state.search_results.len();
		let new_selected;
		if dy.abs() > 1 {
			new_selected = (client_state.selected_result as i32 + dy).clamp(0, files as i32 - 1) as usize;
		} else {
			new_selected = loop_index(client_state.selected_result, dy, files);
		}
		if new_selected != client_state.selected_result {
			client_state.selected_result = new_selected;
			return true;
		}
		false
	}
}