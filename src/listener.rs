use std::{io, time::Duration};
use crossterm::event::{poll, read, Event, KeyEvent};
use mki::Action;

use crate::{component::{block::BlockHandleKey, layer, popup::{PopupHandleGlobalKey, PopupHandleKey, PopupHandlePaste}}, constant::{MIN_HEIGHT, MIN_WIDTH}, state::{self, get_mut_app, SelectionLayer}, util::{notify_redraw, pulseaudio::{play_file, stop_all}, waveform::play_wave}};

pub fn listen_events() -> io::Result<()> {
	let app = state::get_app();
	while app.running {
		if !app.hidden {
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
		app.popups.iter_mut().for_each(|popup| {
			if popup.has_global_key_handler() {
				popup.handle_global_key(key);
				notify_redraw();
			}
		});

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
				if *playing {
					*playing = false;
				}
			}
		});
	}));
}

fn on_resize(width: u16, height: u16) {
	let app = state::get_mut_app();
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
	let app = state::get_mut_app();
	let mut need_redraw = false;
	if !app.error.is_empty() {
		if !app.error_important {
			app.error = String::new();
			need_redraw = true;
		}
	} else if !app.popups.is_empty() {
		need_redraw = app.popups.last_mut()
			.map_or(false, |popup| { popup.handle_key(event) });
	} else {
		need_redraw = match app.selection_layer {
			SelectionLayer::Block => layer::handle_key(event),
			SelectionLayer::Content => app.blocks[app.block_selected as usize].handle_key(event)
		}
	}
	if need_redraw {
		notify_redraw();
	}
}

fn on_paste(data: String) {
	let app = state::get_mut_app();
	if app.popups.iter_mut()
		.map(|popup| { popup.handle_paste(data.clone()) })
		.fold(false, |acc, redraw| { acc || redraw }) {
			notify_redraw();
		}
}