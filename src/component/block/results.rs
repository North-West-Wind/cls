use std::{path::Path, sync::{Arc, Mutex, MutexGuard, OnceLock}, thread::{self, JoinHandle}};

use crossterm::event::KeyCode;
use fuzzy_matcher::{FuzzyMatcher, skim::SkimMatcherV2};
use rand::Rng;
use ratatui::{style::{Color, Modifier, Style}, text::{Line, Span}, widgets::{Block, Borders, Padding, Paragraph}};
use sorted_list::SortedList;
use substring::Substring;
use uuid::Uuid;

use crate::{component::block::{BlockHandleKey, BlockNavigation, BlockRenderArea, BlockSingleton, log, loop_index, search::SearchBlock, settings::SettingsBlock}, state::{acquire, notify_redraw}, util::file::play_file_auto_volume};

enum State {
	Initial,
	Searching,
	Finish
}

#[derive(PartialEq, Clone)]
pub struct FileResult {
	pub parent: String,
	pub name: String,
	duration: String
}

#[derive(PartialEq, Clone)]
pub struct SimpleResult {
	pub uuid: Uuid,
	has_id: bool,
	has_key: bool,
	pub main: String,
	pub sub: String,
}

#[derive(PartialEq, Clone)]
pub enum SearchResult {
	File(FileResult),
	Wave(SimpleResult),
	Dialog(SimpleResult),
}

pub struct ResultsBlock {
	range: (i32, i32),
	height: u16,
	pub selected: usize,
	state: State,
	pub results: SortedList<i64, SearchResult>
}

impl BlockSingleton for ResultsBlock {
	fn instance() -> MutexGuard<'static, Self> {
		static BLOCK: OnceLock<Mutex<ResultsBlock>> = OnceLock::new();
		BLOCK.get_or_init(|| {
			Mutex::new(ResultsBlock {
				range: (-1, -1),
				height: 0,
				selected: 0,
				state: State::Initial,
				results: SortedList::new()
			})
		}).lock().unwrap()
	}
}

impl BlockRenderArea for ResultsBlock {
	fn render_area(&mut self, f: &mut ratatui::prelude::Frame, area: ratatui::prelude::Rect) {
		let app = acquire();
		let (border_type, border_style) = app.borders(Self::ID);
		let block = Block::default()
			.title("Results")
			.borders(Borders::ALL)
			.border_type(border_type)
			.border_style(border_style.fg(if app.block_selected == Self::ID { Color::LightGreen } else { Color::Green }))
			.padding(Padding::new(2, 2, 1, 1));

		if self.range.0 == -1 || self.height != area.height {
			self.range = (0, area.height as i32 - 5);
			self.height = area.height;
		}

		let paragraph = match self.state {
			State::Initial => Paragraph::new("Enter a query to search"),
			State::Searching => Paragraph::new("Searching..."),
			State::Finish => {
				if self.results.len() == 0 {
					Paragraph::new("Result is empty :<")
				} else {
					let mut lines = vec![];
					for (ii, result_type) in self.results.values().enumerate() {
						let mut spans = vec![];
						let (has_id, has_key, main, right, style) = match result_type {
							SearchResult::File(result) => {
								let (parent, name, duration) = (&result.parent, &result.name, &result.duration);
								let full_path = &Path::new(&parent).join(name).into_os_string().into_string().unwrap();
								let entry = app.config.get_file_entry(&full_path);
								let (has_id, has_key) = if entry.is_some() {
									let entry = entry.unwrap();
									(entry.id.is_some(), !entry.keys.is_empty())
								} else {
									(false, false)
								};
								let style = if self.selected == ii {
									Style::default().add_modifier(Modifier::REVERSED)
								} else {
									Style::default()
								};
								(has_id, has_key, name, duration, style)
							},
							SearchResult::Wave(result) => {
								let style = if self.selected == ii {
									Style::default().fg(Color::LightBlue).add_modifier(Modifier::REVERSED)
								} else {
									Style::default().fg(Color::Cyan)
								};
								(result.has_id, result.has_key, &result.main, &result.sub, style)
							},
							SearchResult::Dialog(result) => {
								let style = if self.selected == ii {
									Style::default().fg(Color::LightYellow).add_modifier(Modifier::REVERSED)
								} else {
									Style::default().fg(Color::Yellow)
								};
								(result.has_id, result.has_key, &result.main, &result.sub, style)
							}
						};
						// Construct the line
						if has_id {
							spans.push(Span::from("I").style(Style::default().fg(Color::LightYellow).add_modifier(Modifier::REVERSED)));
						} else {
							spans.push(Span::from(" "));
						}
						if has_key {
							spans.push(Span::from("K").style(Style::default().fg(Color::LightGreen).add_modifier(Modifier::REVERSED)));
						} else {
							spans.push(Span::from(" "));
						}
						spans.push(Span::from(" "));
						let extra = spans.iter()
							.map(|span| { span.width() as i32 })
							.fold(0, |acc, width| { acc + width });
						if main.len() + right.len() + extra as usize > area.width as usize - 6 {
							spans.push(Span::from(main.substring(0, 0.max(area.width as i32 - 10 - extra - right.len() as i32) as usize)).style(style));
							spans.push(Span::from("... ".to_owned() + &right).style(style));
						} else {
							spans.push(Span::from(main.clone()).style(style));
							spans.push(Span::from(vec![" "; 0.max(area.width as i32 - 6 - extra - main.len() as i32 - right.len() as i32) as usize].join("")).style(style));
							spans.push(Span::from(right.clone()).style(style));
						}
						lines.push(Line::from(spans));
					}
					Paragraph::new(lines)
				}
			}
		};
		if self.selected < self.range.0 as usize {
			self.range = (self.selected as i32, self.selected as i32 + area.height as i32 - 5);
		} else if self.selected > self.range.1 as usize {
			self.range = (self.selected as i32 - area.height as i32 + 5, self.selected as i32);
		}
		f.render_widget(paragraph.scroll((self.range.0 as u16, 0)).block(block), area);
	}
}

impl BlockHandleKey for ResultsBlock {
	fn handle_key(&mut self, event: crossterm::event::KeyEvent) -> bool {
		match event.code {
			KeyCode::Up => self.navigate_file(-1),
			KeyCode::Down => self.navigate_file(1),
			KeyCode::Enter => self.play(false),
			KeyCode::Char('/') => self.play(true),
			KeyCode::PageUp => self.navigate_file(-(self.range.1 - self.range.0 + 1)),
			KeyCode::PageDown => self.navigate_file(self.range.1 - self.range.0 + 1),
			KeyCode::Home => self.navigate_file(-i32::MAX),
			KeyCode::End => self.navigate_file(i32::MAX),
			_ => false,
		}
	}
}

impl BlockNavigation for ResultsBlock {
	const ID: u8 = 9;

	fn navigate_block(&self, dx: i16, dy: i16) -> u8 {
		if dy < 0 {
			return SearchBlock::ID;
		}
		if dx > 0 && acquire().settings_opened {
			return SettingsBlock::ID;
		}
		Self::ID
	}
}

impl ResultsBlock {
	pub fn search(&mut self, query: &str) -> JoinHandle<()> {
		log::info(&format!("ResultsBlock is searching up {query}"));
		self.state = State::Searching;
		notify_redraw();
		let query = query.to_owned();
		thread::spawn(move || {
			// Search
			let mut app = acquire();
			app.block_selected = ResultsBlock::ID;
			let matcher = SkimMatcherV2::default();
			let mut block = ResultsBlock::instance();
			block.results = SortedList::new();
			for (tab, files) in &app.files {
				for (file, duration) in files {
					if let Some(score) = matcher.fuzzy_match(file, &query) {
						block.results.insert(-score, SearchResult::File(FileResult {
							parent: tab.clone(),
							name: file.clone(),
							duration: duration.clone()
						}));
					}
				}
			}
			for waveform in &app.waves {
				if let Some(score) = matcher.fuzzy_match(&waveform.label, &query) {
					block.results.insert(-score, SearchResult::Wave(SimpleResult {
						uuid: waveform.uuid,
						has_id: waveform.id.is_some(),
						has_key: !waveform.keys.is_empty(),
						main: waveform.label.clone(),
						sub: waveform.details()
					}));
				}
			}
			for dialog in &app.dialogs {
				if let Some(score) = matcher.fuzzy_match(&dialog.label, &query) {
					block.results.insert(-score, SearchResult::Dialog(SimpleResult {
						uuid: dialog.uuid,
						has_id: dialog.id.is_some(),
						has_key: !dialog.keys.is_empty(),
						main: dialog.label.clone(),
						sub: String::new()
					}));
				}
			}
			block.state = State::Finish;
			notify_redraw();
		})
	}

	pub fn play(&self, random: bool) -> bool {
		if self.results.len() == 0 {
			return false;
		}
		let index;
		if random {
			index = rand::thread_rng().gen_range(0..self.results.len());
		} else {
			if self.selected >= self.results.len() {
				return false;
			}
			index = self.selected;
		}
		match self.results.values().collect::<Vec<_>>()[index] {
			SearchResult::File(result) => {
				let app = acquire();
				let lock = if app.config.playlist_mode {
					app.playlist_lock.clone()
				} else {
					Arc::new(Mutex::new(()))
				};
				play_file_auto_volume(&Path::new(&result.parent).join(&result.name).into_os_string().into_string().unwrap(), lock);
				true
			},
			SearchResult::Wave(result) => {
				let app = acquire();
				let Some(waveform) = app.waves.iter().find(|waveform| waveform.uuid == result.uuid) else { return false };
				waveform.play(true);
				true
			},
			SearchResult::Dialog(result) => {
				let app = acquire();
				let Some(dialog) = app.dialogs.iter().find(|dialog| dialog.uuid == result.uuid) else { return false };
				dialog.play(true);
				true
			}
		}
	}

	fn navigate_file(&mut self, dy: i32) -> bool {
		let files = self.results.len();
		let new_selected;
		if dy.abs() > 1 {
			new_selected = (self.selected as i32 + dy).clamp(0, files as i32 - 1) as usize;
		} else {
			new_selected = loop_index(self.selected, dy, files);
		}
		if new_selected != self.selected {
			self.selected = new_selected;
			return true;
		}
		false
	}
}