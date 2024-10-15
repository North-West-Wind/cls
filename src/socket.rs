use std::{io::{Error, Read, Write}, os::unix::net::{UnixListener, UnixStream}, path::{Path, PathBuf}};

use getopts::Matches;
use normpath::PathExt;
use splitty::split_unquoted_whitespace;

use crate::{config, constant::APP_NAME, state::{get_app, get_mut_app, Scanning}, util::{notify_redraw, threads::spawn_scan_thread}};

fn socket_path() -> PathBuf {
	std::env::temp_dir().join(APP_NAME).join(format!("{APP_NAME}.sock"))
}

pub fn ensure_socket() {
	if !socket_path().exists() {
		let _ = std::fs::create_dir_all(socket_path().parent().unwrap());
		get_mut_app().socket_holder = true;
	}
}

pub fn listen_socket() -> std::io::Result<()> {
	let app = get_app();
	let listener = UnixListener::bind(std::env::temp_dir().join(APP_NAME).join(format!("{APP_NAME}.sock")))?;
	for stream in listener.incoming() {
		if !app.running {
			break;
		}
		match stream {
			Ok(stream) => {
				if handle_stream(stream)? {
					break;
				}
			}
			Err(_) => {}
		}
	}
	std::fs::remove_file(socket_path())?;
	Ok(())
}

pub fn send_exit() -> std::io::Result<()> {
	let mut stream = UnixStream::connect(socket_path())?;
	stream.write_all(b"exit")?;
	Ok(())
}

pub fn send_socket(matches: Matches) -> std::io::Result<()> {
	if !socket_path().exists() {
		return Err(Error::new(std::io::ErrorKind::NotFound, "Socket doens't exist. Isn't there another instance running?"));
	}
	let mut stream = UnixStream::connect(socket_path())?;
	if matches.opt_present("exit") {
		stream.write_all(b"exit")?;
		return Ok(()); // instance should've exited after this, so no more stuff is sent
	}
	if matches.opt_present("reload-config") {
		stream.write_all(b"reload_config")?;
	}
	if matches.opt_present("add-tab") {
		let path = matches.opt_str("add-tab");
		if path.is_some() {
			stream.write_all(format!("add_tab \"{}\"", path.unwrap().trim_matches(|c| c == '\"' || c == '\'')).as_bytes())?;
		}
	}
	if matches.opt_present("delete-current-tab") {
		stream.write_all(b"delete_current_tab")?;
	}
	if matches.opt_present("reload-current-tab") {
		stream.write_all(b"reload_current_tab")?;
	}
	Ok(())
}

fn handle_stream(mut stream: UnixStream) -> std::io::Result<bool> {
	let mut command = String::new();
	stream.read_to_string(&mut command)?;
	let mut token = split_unquoted_whitespace(&command).unwrap_quotes(true);
	match token.next() {
		Some(str) => {
			match str {
				"exit" => {
					return Ok(true);
				},
				"reload_config" => {
					let result = config::load();
					if result.is_ok() {
						notify_redraw();
					}
				},
				"add_tab" => {
					let path = token.next();
					if path.is_some() {
						let path = path.unwrap();
						let app = get_mut_app();
						let norm = Path::new(path).normalize();
						if norm.is_ok() {
							app.config.tabs.push(norm.unwrap().into_os_string().into_string().unwrap());
							app.tab_selected = app.config.tabs.len() - 1;
							spawn_scan_thread(Scanning::One(app.tab_selected));
						}
					}
				},
				"delete_current_tab" => {
					let app = get_mut_app();
					let tab_selected = app.tab_selected;
					let files = app.files.as_mut();
					if files.is_some() {
						files.unwrap().remove(&app.config.tabs[tab_selected]);
						app.config.tabs.remove(tab_selected);
						if app.tab_selected >= app.config.tabs.len() && app.config.tabs.len() != 0 {
							app.tab_selected = app.config.tabs.len() - 1;
						}
						notify_redraw();
					}
				},
				"reload_current_tab" => {
					let app = get_app();
					if app.tab_selected < app.config.tabs.len() {
						spawn_scan_thread(Scanning::One(app.tab_selected));
					}
				},
				_ => (),
			}
		},
		None => (),
	}

	Ok(false)
}