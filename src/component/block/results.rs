use std::{path::Path, sync::{Arc, Mutex, MutexGuard, OnceLock}, thread};

use crossterm::event::KeyCode;
use fuzzy_matcher::{FuzzyMatcher, skim::SkimMatcherV2};
use rand::Rng;
use ratatui::{style::{Color, Modifier, Style}, text::{Line, Span}, widgets::{Block, Borders, Padding, Paragraph}};
use sorted_list::SortedList;
use substring::Substring;

use crate::{component::block::{BlockHandleKey, BlockNavigation, BlockRenderArea, BlockSingleton, loop_index, search::SearchBlock, settings::SettingsBlock}, state::{acquire, notify_redraw}, util::file::play_file_auto_volume};

enum State {
	Initial,
	Searching,
	Finish
}

pub struct ResultsBlock {
	range: (i32, i32),
	height: u16,
	pub selected: usize,
	state: State,
	pub results: SortedList<i64, (String, String)>,
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
					for (ii, (_parent, file)) in self.results.values().enumerate() {
						let mut spans = vec![];
						let style = if self.selected == ii {
							Style::default().add_modifier(Modifier::REVERSED)
						} else {
							Style::default()
						};
						if file.len() as usize > area.width as usize - 6 {
							spans.push(Span::from(file.substring(0, 0.max(area.width as i32 - 10) as usize)).style(style));
							spans.push(Span::from("... ".to_owned()).style(style));
						} else {
							spans.push(Span::from(file.clone()).style(style));
						}
						lines.push(Line::from(spans));
					}
					Paragraph::new(lines)
				}
			}
		};
		f.render_widget(paragraph.block(block), area);
	}
}

impl BlockHandleKey for ResultsBlock {
	fn handle_key(&mut self, event: crossterm::event::KeyEvent) -> bool {
		match event.code {
			KeyCode::Up => self.navigate_file(-1),
			KeyCode::Down => self.navigate_file(1),
			KeyCode::Enter => self.play_file(false),
			KeyCode::Char('/') => self.play_file(true),
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
	pub fn search(&mut self, query: &str) {
		self.state = State::Searching;
		notify_redraw();
		let query = query.to_owned();
		thread::spawn(move || {
			// Search
			let app = acquire();
			let matcher = SkimMatcherV2::default();
			let mut block = ResultsBlock::instance();
			for (tab, files) in &app.files {
				for (file, _duration) in files {
					if let Some(score) = matcher.fuzzy_match(file, &query) {
						block.results.insert(-score, (tab.clone(), file.clone()));
					}
				}
			}
			block.state = State::Finish;
			notify_redraw();
		});
	}

	fn play_file(&self, random: bool) -> bool {
		let index;
		if random {
			index = rand::thread_rng().gen_range(0..self.results.len());
		} else {
			if self.selected >= self.results.len() {
				return false;
			}
			index = self.selected;
		}
		let app = acquire();
		let lock = if app.config.playlist_mode {
			app.playlist_lock.clone()
		} else {
			Arc::new(Mutex::new(()))
		};
		let (parent, file) = self.results.values().collect::<Vec<_>>()[index];
		play_file_auto_volume(&Path::new(parent).join(file).into_os_string().into_string().unwrap(), lock);
		true
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