use std::{cmp::{max, min}, path::Path};

use crossterm::event::{KeyCode, KeyEvent};

use crate::{handler::pulseaudio, state::{get_app, get_mut_app, CondvarPair, Popup, Scanning}, util};

pub fn handle_files(pair: CondvarPair, event: KeyEvent) -> bool {
	let app = get_app();
	if app.scanning == Scanning::ALL || app.scanning == Scanning::ONE(app.tab_selected) {
		return false;
	}
	match event.code {
		KeyCode::Char('r') => reload_tab(pair),
		KeyCode::Up => navigate_file(-1),
		KeyCode::Down => navigate_file(1),
		KeyCode::Enter => play_file(),
		KeyCode::Char('x') => set_global_key_bind(),
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

fn play_file() -> bool {
	let app = get_app();
	if app.files.is_none() {
		return false;
	}
	if app.tab_selected >= app.config.tabs.len() {
		return false;
	}
	let tab = app.config.tabs[app.tab_selected].clone();
	let files = app.files.as_ref().unwrap().get(&tab);
	if files.is_none() {
		return false;
	}
	let unwrapped = files.unwrap();
	if app.file_selected >= unwrapped.len() {
		return false;
	}
	pulseaudio::play_file(&Path::new(&tab).join(&unwrapped[app.file_selected].0).into_os_string().into_string().unwrap());
	return true;
}

fn set_global_key_bind() -> bool {
	let app = get_mut_app();
	app.popup = Popup::KEY_BIND;
	return true;
}