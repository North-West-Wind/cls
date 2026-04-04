use std::{io, sync::{Arc, Mutex}, time::Duration};
use crossterm::event::{Event, KeyEvent, KeyEventKind, poll, read};
use mki::Action;

use crate::{component::{block::{self, log}, layer, popup::{PopupHandleGlobalKey, PopupHandleKey, PopupHandlePaste, popups}}, constant::{MIN_HEIGHT, MIN_WIDTH}, state::{SelectionLayer, acquire, is_running, notify_redraw, stop_running}, util::file::{play_file_auto_volume, stop_all}};

pub fn program_loop() -> io::Result<()> {
	// Global key listener
	log::info("Starting global key listener...");
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
				let lock = if app.config.playlist_mode {
					app.playlist_lock.clone()
				} else {
					Arc::new(Mutex::new(()))
				};
				play_file_auto_volume(path, lock);
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
				wave.play(false);
			} else {
				let mut playing = wave.playing.lock().expect("Failed to lock mutex");
				if !playing.1 {
					playing.0 = false;
				}
			}
		});

		// Dialog hotkey
		app.dialogs.iter().for_each(|dialog| {
			if dialog.keys.len() == 0 {
				return;
			}
			if dialog.keys.iter().all(|key| { key.is_pressed() }) {
				dialog.play(false);
			} else {
				let mut playing = dialog.playing.lock().expect("Failed to lock mutex");
				if !playing.1 {
					playing.0 = false;
				}
			}
		});
	}));

	// Local key listener
	log::info("Starting local key listener...");
	let hidden = { acquire().hidden };
	if hidden {
		while is_running() {
			// This is still required to keep the program from stopping
			std::thread::sleep(Duration::from_millis(500));
		}
	} else {
		while is_running() {
			// `poll()` waits for an `Event` for a given time period
			if poll(Duration::from_millis(500))? {
				// It's guaranteed that the `read()` won't block when the `poll()`
				// function returns `true`
				match read()? {
					//Event::FocusGained => on_focus(true),
					//Event::FocusLost => on_focus(false),
					Event::Key(event) => {
						if event.kind != KeyEventKind::Release {
							on_key(event);
						}
					},
					//Event::Mouse(event) => println!("{:?}", event),
					Event::Paste(data) => on_paste(data),
					Event::Resize(width, height) => on_resize(width, height),
					_ => (),
				}
			}
		}
	}

	// Program has exited
	log::info("Stopping global key listener...");
	mki::remove_any_key_bind();

	// Exit redraw
	notify_redraw();
	Ok(())
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

pub fn listen_signals() {
	log::info("Starting signal listener...");
	ctrlc::set_handler(move || {
		stop_running();
	}).expect("Failed to set Ctrl+C handler");
}