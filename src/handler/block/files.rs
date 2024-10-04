use std::cmp::{max, min};

use crossterm::event::{KeyCode, KeyEvent};

use crate::{state::{get_app, get_mut_app, CondvarPair, Scanning}, util};

pub fn handle_files(pair: CondvarPair, event: KeyEvent) -> bool {
	let app = get_app();
	if app.scanning == Scanning::ALL || app.scanning == Scanning::ONE(app.tab_selected) {
		return false;
	}
	match event.code {
		KeyCode::Char('r') => reload_tab(pair),
		KeyCode::Up => navigate_file(-1),
		KeyCode::Down => navigate_file(1),
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

fn navigate_file(dy: i32) -> bool {
	let app = get_mut_app();
	let files = app.files.as_ref().unwrap().get(&app.config.tabs[app.tab_selected]);
	if files.is_none() {
		return false;
	}
	let new_selected = min(files.unwrap().len() as i32 - 1, max(0, app.file_selected as i32 + dy)) as usize;
	if new_selected != app.file_selected {
		app.file_selected = new_selected;
		return true;
	}
	false
}