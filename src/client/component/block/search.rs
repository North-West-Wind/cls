use std::{thread, vec};

use crate::client::{AtomicClientState, ClientState, FileResult, MainOpened, SearchResult, SearchState, SimpleResult, component::block::{BlockNavigation, info::InfoBlock, results::ResultsBlock}};

use super::{BlockHandleKey, BlockRenderArea};

use crossterm::event::{Event, KeyCode, KeyEvent};
use fuzzy_matcher::{FuzzyMatcher, skim::SkimMatcherV2};
use ratatui::{Frame, layout::Rect, style::Color, widgets::{Block, Borders, Padding, Paragraph}};
use rayon::iter::{IntoParallelRefIterator, ParallelExtend, ParallelIterator};
use tui_input::{Input, backend::crossterm::EventHandler};

#[derive(Default)]
pub struct SearchBlock {
	input: Input,
}

impl BlockRenderArea for SearchBlock {
	fn render_area(&mut self, client_state: &ClientState, f: &mut Frame, area: Rect) {
		let width = area.width as usize - 4;
		let (border_type, border_style) = client_state.borders(Self::ID);
		let block = Block::default()
			.title("Search")
			.borders(Borders::ALL)
			.border_type(border_type)
			.border_style(border_style.fg(if client_state.selected_block == Self::ID { Color::LightGreen } else { Color::Green }));
		let scroll = self.input.visual_scroll(width as usize - 1);
		let paragraph = Paragraph::new(self.input.value()).block(block.padding(Padding::horizontal(1))).scroll((0, scroll as u16));
		f.render_widget(paragraph, area);
		f.set_cursor_position((
			area.x + ((self.input.visual_cursor()).max(scroll) - scroll) as u16 + 2,
			area.y + 1
		));
	}
}

impl BlockHandleKey for SearchBlock {
	fn handle_key(&mut self, atomic_client_state: AtomicClientState, event: KeyEvent) -> bool {
		match event.code {
			KeyCode::Esc => {
				atomic_client_state.write().main_opened = MainOpened::File;
				true
			},
			KeyCode::Enter => {
				{
					let mut client_state = atomic_client_state.write();
					client_state.search_state = SearchState::Searching;
					client_state.selected_block = ResultsBlock::ID;
				}
				let query = self.input.value().to_string();
				thread::spawn(move || {
					use SearchResult::*;
					let matcher = SkimMatcherV2::default();
					let mut results = vec![];
					let client_state = atomic_client_state.read();
					
					// Search file
					results.par_extend(client_state.file_tabs.par_iter().flat_map(|(tab, files)| {
						files.par_iter().filter_map(|(name, info)| {
							if let Some(score) = matcher.fuzzy_match(name, &query) {
								Some((score, File(FileResult {
									parent: tab.clone(),
									name: name.clone(),
									info: info.clone()
								})))
							} else {
								None
							}
						}).collect::<Vec<_>>()
					}));
					// Search waves
					results.par_extend(client_state.waves.par_iter().filter_map(|wave| {
						if let Some(score) = matcher.fuzzy_match(&wave.base.label, &query) {
							Some((score, Wave(SimpleResult {
								uid: wave.base.uid,
								has_id: wave.base.id.is_some(),
								has_keys: !wave.base.keys.is_empty(),
								main: wave.base.label.clone(),
								sub: wave.details()
							})))
						} else {
							None
						}
					}));
					// Search dialogs
					results.par_extend(client_state.dialogs.par_iter().filter_map(|dialog| {
						if let Some(score) = matcher.fuzzy_match(&dialog.label, &query) {
							Some((score, Dialog(SimpleResult {
								uid: dialog.uid,
								has_id: dialog.id.is_some(),
								has_keys: !dialog.keys.is_empty(),
								main: dialog.label.clone(),
								sub: String::new()
							})))
						} else {
							None
						}
					}));
					drop(client_state);
					// Sort
					results.sort_by_key(|(score, _)| -score);
					let mut client_state = atomic_client_state.write();
					client_state.search_state = SearchState::Finish;
					client_state.search_results = results;
				});
				true
			},
			_ => {
				self.input.handle_event(&Event::Key(event));
				true
			}
		}
	}
}

impl BlockNavigation for SearchBlock {
	const ID: u8 = 8;

	fn navigate_block(&self, client_state: AtomicClientState, _dx: i16, dy: i16) -> u8 {
		if dy > 0 {
			return client_state.write().main_opened.id(Self::ID);
		} else if dy < 0 {
			return InfoBlock::ID;
		}
		return Self::ID;
	}
}