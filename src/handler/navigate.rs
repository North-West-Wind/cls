use std::cmp::{max, min};

use crate::state::{get_app, get_mut_app, Popup, SelectionLayer};

pub fn key_navigate(dx: i16, dy: i16) -> bool {
	if dx == 0 && dy == 0 { return false }
	let app = get_app();
	if app.selection_layer == SelectionLayer::BLOCK {
		navigate_block(dx, dy)
	} else {
		false
	}
}

pub fn navigate_block(dx: i16, dy: i16) -> bool {
	if dx == 0 && dy == 0 { return false }
	let app = get_mut_app();
	let old_block = app.block_selected;
	let new_block: i16;
	if dy > 0 {
		// moving down
		new_block = min(2, old_block as i16 + dy);
	} else {
		// moving up
		new_block = max(0, old_block as i16 + dy);
	}

	if new_block as u8 != old_block {
		app.block_selected = new_block as u8;
		return true
	}
	false
}

pub fn navigate_layer(escape: bool) -> bool {
	let app = get_mut_app();
	if escape {
		match app.selection_layer {
			SelectionLayer::BLOCK => {
				navigate_popup(Popup::QUIT);
				return true
			},
			SelectionLayer::CONTENT => {
				set_selection_layer(SelectionLayer::BLOCK);
				return true
			}
		}
	} else {
		match app.selection_layer {
			SelectionLayer::BLOCK => {
				set_selection_layer(SelectionLayer::CONTENT);
				return true
			},
			SelectionLayer::CONTENT => return false,
		}
	}
}

pub fn navigate_popup(popup: Popup) -> bool {
	let app = get_mut_app();
	let old_popup = app.popup;
	if old_popup != popup {
		app.popup = popup;
		return true;
	}
	false
}

fn set_selection_layer(layer: SelectionLayer) {
	let app = get_mut_app();
	if app.selection_layer != layer {
		app.last_selection_layer = app.selection_layer;
		app.selection_layer = layer;
	}
}