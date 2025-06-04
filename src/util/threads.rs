use std::thread;

use crate::{component::popup::{exit_popup, save::SavePopup, set_popup, PopupComponent}, config, state::{get_mut_app, Scanning}, util};

use super::notify_redraw;

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
		notify_redraw();
	});
}

pub fn spawn_save_thread() {
	thread::spawn(move || {
		set_popup(PopupComponent::Save(SavePopup::new(false)));
		notify_redraw();
		let _ = config::save();
		exit_popup();
		set_popup(PopupComponent::Save(SavePopup::new(true)));
		notify_redraw();
	});
}