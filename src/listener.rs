use std::{io, time::Duration, cmp::{min, max}};
use crossterm::event::{poll, read, Event, KeyCode, KeyEvent};

use crate::state::{get_block_selected, get_error, get_running, get_selection_layer, set_block_selected, set_error, set_running, set_selection_layer, CondvarPair, SelectionLayer};

pub fn listen_events(pair: CondvarPair) -> io::Result<()> {
	while get_running() {
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
	if width < 48 || height < 11 {
		set_error(String::from(format!("Terminal size requires at least 48x11.\nCurrent size: {width}x{height}")));
		notify_redraw(pair);
	} else {
		if !get_error().is_empty() {
			set_error(String::new());
			notify_redraw(pair);
		}
	}
}

fn on_key(pair: CondvarPair, event: KeyEvent) {
	let mut need_redraw = false;
	match event.code {
		KeyCode::Up => need_redraw = key_navigate(0, -1),
		KeyCode::Down => need_redraw = key_navigate(0, 1),
		KeyCode::Enter => need_redraw = navigate_layer(false),
		KeyCode::Char('q')|KeyCode::Esc => need_redraw = navigate_layer(true),
		_ => ()
	}
	if need_redraw {
		notify_redraw(pair);
	}
}

fn key_navigate(dx: i16, dy: i16) -> bool {
	if dx == 0 && dy == 0 { return false }
	if get_selection_layer() == SelectionLayer::BLOCK {
		navigate_block(dx, dy)
	} else {
		navigate_content(dx, dy)
	}
}

fn navigate_block(dx: i16, dy: i16) -> bool {
	if dx == 0 && dy == 0 { return false }
	let old_block = get_block_selected();
	let new_block: i16;
	if dy > 0 {
		// moving down
		new_block = min(2, get_block_selected() as i16 + dy);
	} else {
		// moving up
		new_block = max(0, get_block_selected() as i16 + dy);
	}

	if new_block as u8 != old_block {
		set_block_selected(new_block as u8);
		return true
	}
	false
}

fn navigate_content(dx: i16, dy: i16) -> bool {
	false
}

fn navigate_layer(escape: bool) -> bool {
	if escape {
		match get_selection_layer() {
			SelectionLayer::BLOCK => {
				set_running(false);
				return false
			},
			SelectionLayer::CONTENT => {
				set_selection_layer(SelectionLayer::BLOCK);
				return true
			}
		}
	} else {
		match get_selection_layer() {
			SelectionLayer::BLOCK => {
				set_selection_layer(SelectionLayer::CONTENT);
				return true
			},
			SelectionLayer::CONTENT => return false,
		}
	}
}