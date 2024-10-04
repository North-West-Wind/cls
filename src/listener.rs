use std::{io, time::Duration};
use crossterm::event::{poll, read, Event, KeyEvent};

use crate::{handler::{block::handle_block_key_event, input::handle_input_key_event, layer::handle_layer_key_event, popup::handle_popup_key_event}, state::{self, CondvarPair, InputMode, Popup, SelectionLayer}};

pub fn listen_events(pair: CondvarPair) -> io::Result<()> {
	let app = state::get_app();
	while app.running {
		// `poll()` waits for an `Event` for a given time period
		if poll(Duration::from_millis(500))? {
				// It's guaranteed that the `read()` won't block when the `poll()`
				// function returns `true`
				match read()? {
						//Event::FocusGained => println!("FocusGained"),
						//Event::FocusLost => println!("FocusLost"),
						Event::Key(event) => on_key(pair.clone(), event),
						//Event::Mouse(event) => println!("{:?}", event),
						//Event::Paste(data) => println!("Pasted {:?}", data),
						Event::Resize(width, height) => on_resize(pair.clone(), width, height),
						_ => (),
				}
		}
	}
	notify_redraw(pair);
	Ok(())
}

fn notify_redraw(pair: CondvarPair) {
	let (lock, cvar) = &*pair;
	let mut shared = lock.lock().unwrap();
	(*shared).redraw = true;
	cvar.notify_all();
}

fn on_resize(pair: CondvarPair, width: u16, height: u16) {
	let app = state::get_mut_app();
	if width < 48 || height < 11 {
		app.error = String::from(format!("Terminal size requires at least 48x11.\nCurrent size: {width}x{height}"));
		notify_redraw(pair);
	} else {
		if !app.error.is_empty() {
			app.error = String::new();
			notify_redraw(pair);
		}
	}
}

fn on_key(pair: CondvarPair, event: KeyEvent) {
	let app = state::get_app();
	let need_redraw: bool;
	if app.input_mode == InputMode::EDITING {
		need_redraw = handle_input_key_event(event);
	} else if app.popup != Popup::NONE {
		need_redraw = handle_popup_key_event(event);
	} else {
		need_redraw = match app.selection_layer {
			SelectionLayer::BLOCK => handle_layer_key_event(event),
			SelectionLayer::CONTENT => handle_block_key_event(event)
		}
	}
	if need_redraw {
		notify_redraw(pair);
	}
}