use std::{ptr::{addr_of, addr_of_mut}, sync::{Arc, Condvar, Mutex}};

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

pub struct App {
	pub config: SoundboardConfig,
	pub running: bool,
	pub error: String,
	pub selection_layer: SelectionLayer,
	pub block_selected: u8,
	pub popup: Popup,
	pub last_selection_layer: SelectionLayer,
	pub module_num: String,
	pub input: Option<Input>,
	pub input_mode: InputMode,
	pub await_input: AwaitInput,
	pub tab_selected: usize,
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
		config: create_config(),
		running: false,
		error: String::new(),
		selection_layer: SelectionLayer::BLOCK,
		block_selected: 0,
		popup: Popup::NONE,
		last_selection_layer: SelectionLayer::BLOCK,
		module_num: String::new(),
		input: Option::None,
		input_mode: InputMode::NORMAL,
		await_input: AwaitInput::NONE,
		tab_selected: 0,
	}
}

pub fn get_mut_app() -> &'static mut App {
	unsafe { &mut *(addr_of_mut!(APP)) }
}

pub fn get_app() -> &'static App {
	unsafe { &*(addr_of!(APP)) }
}