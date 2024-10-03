use std::sync::{Arc, Condvar, Mutex};

pub type CondvarPair = Arc<(Mutex<SharedCondvar>, Condvar)>;

pub struct SharedCondvar {
	pub redraw: bool,
}

pub fn init_shared_condvar() -> SharedCondvar {
	return SharedCondvar {
		redraw: true,
	}
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum SelectionLayer {
	BLOCK,
	CONTENT,
	POPUP
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum Popup {
	NONE,
	HELP,
	QUIT,
	INPUT
}

// Global variables
static mut RUNNING: bool = false;
static mut ERROR: String = String::new();
static mut SELECTION_LAYER: SelectionLayer = SelectionLayer::BLOCK;
static mut BLOCK_SELECTED: u8 = 0;
static mut POPUP: Popup = Popup::NONE;
static mut LAST_SELECTION_LAYER: SelectionLayer = SelectionLayer::BLOCK;

pub fn get_running() -> bool {
	unsafe { RUNNING }
}

pub fn set_running(b: bool) {
	unsafe { RUNNING = b };
}

pub fn get_error() -> String {
	unsafe { ERROR.clone() }
}

pub fn set_error(s: String) {
	unsafe { ERROR = s };
}

pub fn get_selection_layer() -> SelectionLayer {
	unsafe { SELECTION_LAYER }
}

pub fn set_selection_layer(s: SelectionLayer) {
	unsafe {
		if SELECTION_LAYER != s {
			LAST_SELECTION_LAYER = SELECTION_LAYER;
			SELECTION_LAYER = s;
		}
	}
}

pub fn get_block_selected() -> u8 {
	unsafe { BLOCK_SELECTED }
}

pub fn set_block_selected(u: u8) {
	unsafe { BLOCK_SELECTED = u };
}

pub fn get_popup() -> Popup {
	unsafe { POPUP }
}

pub fn set_popup(p: Popup) {
	unsafe { POPUP = p };
}

pub fn get_last_selection_layer() -> SelectionLayer {
	unsafe { LAST_SELECTION_LAYER }
}