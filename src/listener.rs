use std::{io, time::Duration};
use crossterm::event::{poll, read, Event, KeyEvent};
use mki::Action;

use crate::{component::{block, layer, popup::{PopupHandleGlobalKey, PopupHandleKey, PopupHandlePaste, popups}}, constant::{MIN_HEIGHT, MIN_WIDTH}, state::{SelectionLayer, acquire, acquire_running, notify_redraw}, util::{file::{play_file, stop_all}, waveform::play_wave}};

pub fn listen_events() -> io::Result<()> {
	let hidden = { acquire().hidden };
	if hidden {
		while *acquire_running() {
			// This is still required to keep the program from stopping
			std::thread::sleep(Duration::from_millis(500));
		}
	} else {
		while *acquire_running() {
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
		}
	}
	// Exit redraw
	notify_redraw();
	Ok(())
}

pub fn listen_global() {
	mki::bind_any_key(Action::handle_kb(move |key| {
		popups().iter_mut().for_each(|popup| {
			if popup.has_global_key_handler() {
				popup.handle_global_key(key);
				notify_redraw();
			}
		});

		let app = acquire();
		// File hotkey
		app.hotkey.iter().for_each(|(path, keys)| {
			if keys.iter().all(|key| { key.is_pressed() }) {
				play_file(path);
			}
		});
		if !app.stopkey.is_empty() && !app.edit {
			if app.stopkey.iter().all(|key| { key.is_pressed() }) {
				stop_all();
			}
		}

		// Waveform hotkey
		app.waves.iter().for_each(|wave| {
			if wave.keys.len() == 0 {
				return;
			}
			if wave.keys.iter().all(|key| { key.is_pressed() }) {
				play_wave(wave.clone(), false);
			} else {
				let mut playing = wave.playing.lock().expect("Failed to lock mutex");
				if !playing.1 {
					playing.0 = false;
				}
			}
		});
	}));
}

pub fn unlisten_global() {
	mki::remove_any_key_bind();
}

fn on_resize(width: u16, height: u16) {
	let mut app = acquire();
	if width < MIN_WIDTH || height < MIN_HEIGHT {
		app.error = String::from(format!("Terminal size requires at least {MIN_WIDTH}x{MIN_HEIGHT}.\nCurrent size: {width}x{height}"));
		app.error_important = true;
	} else {
		if !app.error.is_empty() {
			app.error = String::new();
			app.error_important = false;
		}
	}
	notify_redraw();
}

fn on_key(event: KeyEvent) {
	let (error, error_important, block_selected, selection_layer) = {
		let app = acquire();
		(app.error.clone(), app.error_important, app.block_selected, app.selection_layer)
	};
	let mut popups = popups();
	let mut need_redraw = false;
	if !error.is_empty() {
		if !error_important {
			acquire().error = String::new();
			need_redraw = true;
		}
	} else if !popups.is_empty() {
		need_redraw = popups.last_mut()
			.map_or(false, |popup| { popup.handle_key(event) });
	} else {
		drop(popups);
		need_redraw = match selection_layer {
			SelectionLayer::Block => layer::handle_key(event),
			SelectionLayer::Content => block::handle_key(block_selected, event)
		}
	}
	if need_redraw {
		notify_redraw();
	}
}

fn on_paste(data: String) {
	if popups().iter_mut()
		.map(|popup| { popup.handle_paste(data.clone()) })
		.fold(false, |acc, redraw| { acc || redraw }) {
			notify_redraw();
		}
}