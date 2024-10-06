use std::thread;

use crate::{component::popup::{save::SavePopup, set_popup, PopupComponent}, config, state::{get_app, get_mut_app, Scanning}, util};

pub fn spawn_scan_thread(mode: Scanning) {
	if mode == Scanning::None {
		return;
	}
	thread::spawn(move || {
		let app = get_mut_app();
		app.scanning = mode;
		let _ = match mode {
				Scanning::All => util::scan_tabs(),
				Scanning::One(index) => util::scan_tab(index),
				_ => Ok(())
		};
		app.scanning = Scanning::None;
		let pair = app.pair.as_ref().unwrap().clone();
		let (lock, cvar) = &*pair;
		let mut shared = lock.lock().unwrap();
		(*shared).redraw = true;
		cvar.notify_all();
	});
}

pub fn spawn_save_thread() {
	thread::spawn(move || {
		let app = get_app();
		let pair = app.pair.as_ref().unwrap().clone();
		set_popup(PopupComponent::Save(SavePopup::new(false)));
		let (lock, cvar) = &*pair;
		let mut shared = lock.lock().unwrap();
		(*shared).redraw = true;
		cvar.notify_all();
		std::mem::drop(shared);
		let _ = config::save();
		set_popup(PopupComponent::Save(SavePopup::new(true)));
		let (lock, cvar) = &*pair;
		let mut shared = lock.lock().unwrap();
		(*shared).redraw = true;
		cvar.notify_all();
	});
}