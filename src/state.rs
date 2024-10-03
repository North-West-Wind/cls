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
	CONTENT
}

// Global variables
static mut RUNNING: bool = false;
static mut ERROR: String = String::new();
static mut SELECTION_LAYER: SelectionLayer = SelectionLayer::BLOCK;
static mut BLOCK_SELECTED: u8 = 0;

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
	unsafe { SELECTION_LAYER = s };
}

pub fn get_block_selected() -> u8 {
	unsafe { BLOCK_SELECTED }
}

pub fn set_block_selected(u: u8) {
	unsafe { BLOCK_SELECTED = u };
}