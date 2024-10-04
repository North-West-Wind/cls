use crossterm::event::{KeyCode, KeyEvent};

use crate::{state::{get_app, get_mut_app, CondvarPair, Scanning}, util::{self, scan_tab}};

pub fn handle_files(pair: CondvarPair, event: KeyEvent) -> bool {
	let app = get_app();
	if app.scanning == Scanning::ALL || app.scanning == Scanning::ONE(app.tab_selected) {
		return false;
	}
	match event.code {
		KeyCode::Char('r') => reload_tab(pair),
		_ => false,
	}
}

fn reload_tab(pair: CondvarPair) -> bool {
	let app = get_app();
	if app.tab_selected < app.config.tabs.len() {
		spawn_scan_thread(pair);
		return true;
	}
	false
}

fn spawn_scan_thread(pair: CondvarPair) {
	std::thread::spawn(move || {
			let app = get_mut_app();
			app.scanning = Scanning::ONE(app.tab_selected);
			let (lock, cvar) = &*pair;
			let mut shared = lock.lock().unwrap();
			(*shared).redraw = true;
			cvar.notify_all();
			std::mem::drop(shared);
			let _ = util::scan_tab(app.tab_selected);
			app.scanning = Scanning::NONE;
			let (lock, cvar) = &*pair;
			let mut shared = lock.lock().unwrap();
			(*shared).redraw = true;
			cvar.notify_all();
	});
}