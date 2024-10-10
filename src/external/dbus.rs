use std::{error::Error, path::Path, thread};

use event_listener::{Event, Listener};
use normpath::PathExt;
use zbus::{blocking::connection, interface};

use crate::{state::{get_app, get_mut_app, Scanning}, util::threads::spawn_scan_thread};

struct DBusHandler {
	done: Event
}

#[interface(name = "in.northwestw.cls.interface")]
impl DBusHandler {
	fn add_tab(&self, path: &str) -> bool {
		let app = get_mut_app();
		let norm = Path::new(path).normalize();
		if norm.is_err() {
			return false;
		}
		app.config.tabs.push(norm.unwrap().into_os_string().into_string().unwrap());
		app.tab_selected = app.config.tabs.len() - 1;
		spawn_scan_thread(Scanning::One(app.tab_selected));
		true
	}

	fn delete_current_tab(&self) -> bool {
		let app = get_mut_app();
		let tab_selected = app.tab_selected;
		let files = app.files.as_mut();
		if files.is_none() {
			return false;
		}
		let val = files.unwrap().remove(&app.config.tabs[tab_selected]);
		app.config.tabs.remove(tab_selected);
		if app.tab_selected >= app.config.tabs.len() && app.config.tabs.len() != 0 {
			app.tab_selected = app.config.tabs.len() - 1;
		}
		val.is_some()
	}

	fn reload_current_tab(&self) -> bool {
		let app = get_app();
		if app.tab_selected < app.config.tabs.len() {
			spawn_scan_thread(Scanning::One(app.tab_selected));
			return true;
		}
		false
	}
}

pub fn start_zbus() {
	thread::spawn(move || {
		let _ = start_listener();
	});
}

fn start_listener() -> Result<(), Box<dyn Error>> {
	let handler = DBusHandler {
		done: Event::new(),
	};
	let listener = handler.done.listen();
	let _conn = connection::Builder::session()?
		.name("in.northwestw.cls")?
		.serve_at("/in/northwestw/cls", handler)?
		.build()?;

	listener.wait();
	Ok(())
}