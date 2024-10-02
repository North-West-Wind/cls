use std::{io, time::Duration, sync::{Arc, Mutex, Condvar}};
use crossterm::event::{poll, read, Event};

use crate::state;

pub fn listen_events(pair: Arc<(Mutex<bool>, Condvar)>) -> io::Result<()> {
	while unsafe { state::RUNNING } {
		// `poll()` waits for an `Event` for a given time period
		if poll(Duration::from_millis(500))? {
				// It's guaranteed that the `read()` won't block when the `poll()`
				// function returns `true`
				match read()? {
						//Event::FocusGained => println!("FocusGained"),
						//Event::FocusLost => println!("FocusLost"),
						//Event::Key(event) => println!("{:?}", event),
						//Event::Mouse(event) => println!("{:?}", event),
						//Event::Paste(data) => println!("Pasted {:?}", data),
						Event::Resize(width, height) => on_resize(pair.clone(), width, height),
						_ => (),
				}
		} else {
				// Timeout expired and no `Event` is available
		}
	}
	notify_redraw(pair);
	Ok(())
}

fn notify_redraw(pair: Arc<(Mutex<bool>, Condvar)>) {
	let (lock, cvar) = &*pair;
	let mut redraw = lock.lock().unwrap();
	*redraw = true;
	cvar.notify_one();
}

fn on_resize(pair: Arc<(Mutex<bool>, Condvar)>, width: u16, height: u16) {
	unsafe { state::ERROR = String::from(format!("Size: {width}x{height}")) };
	notify_redraw(pair);
}