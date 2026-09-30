use std::{format, time::Duration};
use crossterm::event::{Event, KeyEvent, KeyEventKind, poll, read};
use mki::Action;

use crate::{client::{AtomicClientState, AtomicStaticBlocks, ClientState, SelectionLayer, component::{block, layer, popup::{PopupHandleGlobalKey, PopupHandleKey, PopupHandlePaste}}}, common::{constant::{MIN_HEIGHT, MIN_WIDTH}, log}};

pub fn init_key_listener(client_state: AtomicClientState, blocks: AtomicStaticBlocks) -> Result<(), Box<dyn std::error::Error>> {
	// Global key listener
	log::info("Starting global key listener...".to_string());
	let (popup_manager, redrawer) = {
		let client_state = client_state.read();
		(client_state.popup_manager.clone(), client_state.redrawer.clone())
	};
	mki::bind_any_key(Action::handle_kb(move |key| {
		let mut popups = popup_manager.popups.lock();
		let last_popup = popups.last_mut();
		if let Some(popup) = last_popup && popup.has_global_key_handler() {
			popup.handle_global_key(key);
			redrawer.notify();
		}
	}));

	// Local key listener
	log::info("Starting local key listener...".to_string());
	while client_state.read().running {
		// `poll()` waits for an `Event` for a given time period
		if poll(Duration::from_millis(500))? {
			// It's guaranteed that the `read()` won't block when the `poll()`
			// function returns `true`
			match read()? {
				//Event::FocusGained => on_focus(true),
				//Event::FocusLost => on_focus(false),
				Event::Key(event) => {
					if event.kind != KeyEventKind::Release {
						on_key(client_state.clone(), blocks.clone(), event);
					}
				},
				//Event::Mouse(event) => println!("{:?}", event),
				Event::Paste(data) => on_paste(&client_state.read(), data),
				Event::Resize(width, height) => on_resize(client_state.clone(), width, height),
				_ => (),
			}
		}
	}

	Ok(())
}

fn on_resize(client_state: AtomicClientState, width: u16, height: u16) {
	let mut client_state = client_state.write();
	if width < MIN_WIDTH || height < MIN_HEIGHT {
		client_state.error = String::from(format!("Terminal size requires at least {MIN_WIDTH}x{MIN_HEIGHT}.\nCurrent size: {width}x{height}"));
		client_state.error_important = true;
	} else {
		if !client_state.error.is_empty() {
			client_state.error = String::new();
			client_state.error_important = false;
		}
	}
	client_state.redrawer.notify();
}

fn on_key(client_state: AtomicClientState, blocks: AtomicStaticBlocks, event: KeyEvent) {
	let (error, error_important, selection_layer, popup_manager) = {
		let client_state = client_state.read();
		(client_state.error.clone(), client_state.error_important, client_state.selection_layer, client_state.popup_manager.clone())
	};
	let mut need_redraw = false;
	if !error.is_empty() {
		if !error_important {
			{ client_state.write().error = String::new() };
			need_redraw = true;
		}
	} else if let mut popups = popup_manager.popups.lock() && !popups.is_empty() {
		need_redraw = popups.last_mut()
			.map_or(false, |popup| { popup.handle_key(&mut client_state.write(), event) });
	} else {
		need_redraw = match selection_layer {
			SelectionLayer::Block => layer::handle_key(client_state.clone(), blocks, event),
			SelectionLayer::Content => block::handle_key(client_state.clone(), blocks, event)
		}
	}
	if need_redraw {
		client_state.read().redrawer.notify();
	}
}

fn on_paste(client_state: &ClientState, data: String) {
	let (popup_manager, redrawer) = (client_state.popup_manager.clone(), client_state.redrawer.clone());
	let mut popups = popup_manager.popups.lock();
	let last_popup = popups.last_mut();
	if let Some(popup) = last_popup && popup.handle_paste(data) {
		redrawer.notify();
	}
}