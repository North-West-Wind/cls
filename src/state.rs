use std::{collections::HashMap, ptr::{addr_of, addr_of_mut}, sync::{Arc, Condvar, Mutex}};

use mki::Keyboard;
use pulsectl::controllers::SinkController;
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
pub enum AwaitInput {
	None,
	AddTab,
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
	// pulseaudio
	pub module_num: String,
	pub sink_controller: Option<SinkController>,
	// input
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
	pub playing: Option<HashMap<Uuid, (String, u32)>>,
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
		// pulseaudio
		sink_controller: Option::None,
		module_num: String::new(),
		// input
		await_input: AwaitInput::None,
		// render states: volume
		volume_selected: 0,
		// render states: tab
		tab_selected: 0,
		// render states: files
		files: Option::None,
		file_selected: 0,
		scanning: Scanning::None,
		// render states: playing
		playing: Option::None,
	}
}

pub fn get_mut_app() -> &'static mut App {
	unsafe { &mut *(addr_of_mut!(APP)) }
}

pub fn get_app() -> &'static App {
	unsafe { &*(addr_of!(APP)) }
}