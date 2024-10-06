use std::{io, time::Duration};
use crossterm::event::{poll, read, Event, KeyEvent};
use mki::Action;

use crate::{component::{block::BlockHandleKey, layer, popup::{PopupHandleGlobalKey, PopupHandleKey, PopupHandlePaste}}, constant::{MIN_HEIGHT, MIN_WIDTH}, state::{self, get_mut_app, SelectionLayer}, util::{notify_redraw, pulseaudio::play_file}};

pub fn listen_events(no_listen: bool) -> io::Result<()> {
	let app = state::get_app();
	while app.running {
		if !no_listen {
			// `poll()` waits for an `Event` for a given time period
			if poll(Duration::from_millis(500))? {
				// It's guaranteed that the `read()` won't block when the `poll()`
				// function returns `true`
				match read()? {
					//Event::FocusGained => on_focus(true),
					//Event::FocusLost => on_focus(false),
					Event::Key(event) => on_key(event),
					//Event::Mouse(event) => println!("{:?}", event),
					Event::Paste(data) => on_paste(data),
					Event::Resize(width, height) => on_resize(width, height),
					_ => (),
				}
			}
		} else {
			// this is still required to keep the program from stopping
			std::thread::sleep(Duration::from_millis(500));
		}
	}
	notify_redraw();
	Ok(())
}

pub fn listen_global_input() {
	mki::bind_any_key(Action::handle_kb(move |key| {
		let app = get_mut_app();
		if app.popup.as_ref().is_some_and(|popup| { popup.has_global_key_handler() }) {
			app.popup.as_mut().unwrap().handle_global_key(key);
			notify_redraw();
		} else {
			for (path, keys) in app.hotkey.as_ref().unwrap() {
				if keys.iter().all(|key| { key.is_pressed() }) {
					play_file(path);
				}
			}
		}
	}));
}

fn on_resize(width: u16, height: u16) {
	let app = state::get_mut_app();
	if width < MIN_WIDTH || height < MIN_HEIGHT {
		app.error = String::from(format!("Terminal size requires at least {MIN_WIDTH}x{MIN_HEIGHT}.\nCurrent size: {width}x{height}"));
	} else {
		if !app.error.is_empty() {
			app.error = String::new();
		}
	}
	notify_redraw();
}

fn on_key(event: KeyEvent) {
	let app = state::get_mut_app();
	let need_redraw: bool;
	if app.popup.is_some() {
		need_redraw = app.popup.as_mut().unwrap().handle_key(event);
	} else {
		need_redraw = match app.selection_layer {
			SelectionLayer::Block => layer::handle_key(event),
			SelectionLayer::Content => (&app.blocks[app.block_selected as usize]).handle_key(event)
		}
	}
	if need_redraw {
		notify_redraw();
	}
}

fn on_paste(data: String) {
	let app = state::get_mut_app();
	if app.popup.is_some() {
		if app.popup.as_mut().unwrap().handle_paste(data) {
			notify_redraw();
		}
	}
}