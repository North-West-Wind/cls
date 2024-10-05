use std::{collections::{HashMap, HashSet, VecDeque}, ptr::{addr_of, addr_of_mut}, sync::{Arc, Condvar, Mutex}};

use mki::Keyboard;
use pulsectl::controllers::SinkController;
use tui_input::Input;

use crate::config::{create_config, SoundboardConfig};

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
	BLOCK,
	CONTENT
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum Popup {
	NONE,
	HELP,
	QUIT,
	DELETE_TAB,
	KEY_BIND,
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum InputMode {
	NORMAL,
	EDITING
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum AwaitInput {
	NONE,
	ADD_TAB,
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum Scanning {
	NONE,
	ALL,
	ONE(usize)
}

pub struct App {
	// config
	pub config: SoundboardConfig,
	pub hotkey: Option<HashMap<String, Vec<Keyboard>>>,
	// states
	pub running: bool,
	pub error: String,
	pub pair: Option<CondvarPair>,
	// render states: root
	pub block_selected: u8,
	pub selection_layer: SelectionLayer,
	pub last_selection_layer: SelectionLayer,
	pub popup: Popup,
	// pulseaudio
	pub module_num: String,
	pub sink_controller: Option<SinkController>,
	// input
	pub input: Option<Input>,
	pub input_mode: InputMode,
	pub await_input: AwaitInput,
	// render states: volume
	pub volume_selected: usize,
	// render states: tab
	pub tab_selected: usize,
	// render states: files
	pub files: Option<HashMap<String, Vec<(String, String)>>>,
	pub file_selected: usize,
	pub scanning: Scanning,
	// render states: playing
	pub playing: VecDeque<String>,
	// render states: key bind popup
	pub recording: bool,
	pub recorded: Option<HashSet<Keyboard>>,
	// render states: scroll range
	pub tabs_range: (i32, i32),
	pub files_range: (i32, i32),
}

impl Default for App {
	fn default() -> Self {
		create_app()
	}
}

// Global variables
static mut APP: App = create_app();

const fn create_app() -> App {
	App {
		// config
		config: create_config(),
		hotkey: Option::None,
		// states
		running: false,
		error: String::new(),
		pair: Option::None,
		// render states: root
		block_selected: 0,
		selection_layer: SelectionLayer::BLOCK,
		last_selection_layer: SelectionLayer::BLOCK,
		popup: Popup::NONE,
		// pulseaudio
		sink_controller: Option::None,
		module_num: String::new(),
		// input
		input: Option::None,
		input_mode: InputMode::NORMAL,
		await_input: AwaitInput::NONE,
		// render states: volume
		volume_selected: 0,
		// render states: tab
		tab_selected: 0,
		// render states: files
		files: Option::None,
		file_selected: 0,
		scanning: Scanning::NONE,
		// render states: playing
		playing: VecDeque::new(),
		// render states: key bind popup
		recording: false,
		recorded: Option::None,
		// render states: scroll range
		tabs_range: (-1, -1),
		files_range: (-1, -1),
	}
}

pub fn get_mut_app() -> &'static mut App {
	unsafe { &mut *(addr_of_mut!(APP)) }
}

pub fn get_app() -> &'static App {
	unsafe { &*(addr_of!(APP)) }
}