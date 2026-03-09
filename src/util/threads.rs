use std::{io, thread::{self, JoinHandle}};

use crate::{component::{block::log, popup::{PopupComponent, exit_popup, save::SavePopup, set_popup}}, config, socket::{listen_socket, try_socket}, state::{Scanning, acquire, notify_redraw}, util};

// A thread for listening for socket (IPC)
pub fn spawn_socket_thread() -> Result<JoinHandle<()>, io::Error> {
	let listener = try_socket()?;
	log::info("Spawning socket thread...");

	Ok(thread::spawn(move || {
		{ acquire().socket_holder = true; }
		listen_socket(listener);
	}))
}

pub fn spawn_scan_thread(mode: Scanning) {
	if mode == Scanning::None {
		return;
	}
	thread::spawn(move || {
		{ acquire().scanning = mode; }
		match mode {
			Scanning::All => {
				log::info("Scanning all tabs...");
				let _ = util::scan_tabs();
				log::info("Scanned all tabs");
			},
			Scanning::One(index) => {
				log::info(format!("Scanning tab {}...", index).as_str());
				util::scan_tab(index);
				log::info(format!("Scanned tab {}", index).as_str());
			},
			_ => ()
		};

		let mut app = acquire();
		app.scanning = Scanning::None;
		notify_redraw();
	});
}

pub fn spawn_save_thread() {
	thread::spawn(move || {
		set_popup(PopupComponent::Save(SavePopup::new(false)));
		notify_redraw();
		config::save();
		exit_popup();
		set_popup(PopupComponent::Save(SavePopup::new(true)));
		notify_redraw();
	});
}