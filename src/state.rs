use std::{collections::HashMap, ptr::{addr_of, addr_of_mut}, sync::{Arc, Condvar, Mutex}};

use mki::Keyboard;
use pulsectl::controllers::SinkController;
use std_semaphore::Semaphore;
use uuid::Uuid;

use crate::{component::{block::BlockComponent, popup::PopupComponent}, config::{create_config, SoundboardConfig}};

pub type CondvarPair = Arc<(Mutex<SharedCondvar>, Condvar)>;

pub struct SharedCondvar {
	pub redraw: bool,
}

impl Default for SharedCondvar {
	fn default() -> Self {
		Self {
			redraw: true
		}
	}
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum SelectionLayer {
	Block,
	Content
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum Scanning {
	None,
	All,
	One(usize)
}

pub struct App {
	// config
	pub config: SoundboardConfig,
	pub hotkey: Option<HashMap<String, Vec<Keyboard>>>,
	pub stopkey: Option<Vec<Keyboard>>,
	// states
	pub running: bool,
	pub error: String,
	pub error_important: bool,
	pub pair: Option<CondvarPair>,
	pub socket_holder: bool,
	pub hidden: bool,
	pub edit: bool,
	// render states: root
	pub blocks: Vec<BlockComponent>,
	pub block_selected: u8,
	pub selection_layer: SelectionLayer,
	pub popup: Option<PopupComponent>,
	pub settings_opened: bool,
	// pulseaudio
	pub module_nums: Vec<String>,
	pub sink_controller: Option<SinkController>,
	// render states: files
	pub files: Option<HashMap<String, Vec<(String, String)>>>,
	pub scanning: Scanning,
	// render states: playing
	pub playing_file: Option<HashMap<Uuid, String>>,
	pub playing_process: Option<HashMap<Uuid, u32>>,
	pub playing_semaphore: Option<Semaphore>,
}

impl Default for App {
	fn default() -> Self {
		create_app()
	}
}

impl App {
	pub fn file_selected(&self) -> usize {
		self.blocks[2].file_selected().unwrap()
	}

	pub fn set_file_selected(&mut self, selected: usize) {
		self.blocks[2].set_file_selected(selected);
	}

	pub fn tab_selected(&self) -> usize {
		self.blocks[1].tab_selected().unwrap()
	}

	pub fn set_tab_selected(&mut self, selected: usize) {
		self.blocks[1].set_tab_selected(selected);
	}
}

// Global variables
static mut APP: App = create_app();

const fn create_app() -> App {
	App {
		// config
		config: create_config(),
		hotkey: Option::None,
		stopkey: Option::None,
		// states
		running: false,
		error: String::new(),
		error_important: false,
		pair: Option::None,
		socket_holder: false,
		hidden: false,
		edit: false,
		// render states: root
		blocks: vec![],
		block_selected: 0,
		selection_layer: SelectionLayer::Block,
		popup: Option::None,
		settings_opened: false,
		// pulseaudio
		sink_controller: Option::None,
		module_nums: vec![],
		// render states: files
		files: Option::None,
		scanning: Scanning::None,
		// render states: playing
		playing_file: Option::None,
		playing_process: Option::None,
		playing_semaphore: Option::None,
	}
}

pub fn get_mut_app() -> &'static mut App {
	unsafe { &mut *(addr_of_mut!(APP)) }
}

pub fn get_app() -> &'static App {
	unsafe { &*(addr_of!(APP)) }
}