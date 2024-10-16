use std::{io::{Error, Read, Write}, os::unix::net::{UnixListener, UnixStream}, path::{Path, PathBuf}};

use clap::ArgMatches;
use code::SocketCode;
use normpath::PathExt;

use crate::{config, constant::APP_NAME, state::{get_app, get_mut_app, Scanning}, util::{notify_redraw, threads::spawn_scan_thread}};

pub mod code;

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
	if !socket_path().exists() {
		return Ok(());
	}
	let mut stream = UnixStream::connect(socket_path())?;
	stream.write(&[SocketCode::Exit.to_u8()])?;
	Ok(())
}

pub fn send_socket(subcommand: (&str, &ArgMatches)) -> std::io::Result<()> {
	if !socket_path().exists() {
		return Err(Error::new(std::io::ErrorKind::NotFound, "Socket doens't exist. Isn't there another instance running?"));
	}
	let code = SocketCode::from_str(subcommand.0);
	if code.is_none() {
		return Err(Error::new(std::io::ErrorKind::NotFound, "Invalid command"));
	}
	let stream = UnixStream::connect(socket_path())?;
	code.unwrap().write_to_stream(stream, subcommand.1)?;
	Ok(())
}

fn handle_stream(mut stream: UnixStream) -> std::io::Result<bool> {
	use SocketCode::*;
	let mut code = [0];
	stream.read_exact(&mut code)?;
	let code = SocketCode::from_u8(code[0]);
	if code.is_none() {
		return Ok(false);
	}
	match code.unwrap() {
		Exit => {
			get_mut_app().running = false;
			return Ok(true);
		},
		ReloadConfig => {
			let result = config::load();
			if result.is_ok() {
				notify_redraw();
			}
		},
		AddTab => {
			let mut path = String::new();
			stream.read_to_string(&mut path)?;
			if !path.is_empty() {
				let app = get_mut_app();
				let norm = Path::new(&path).normalize();
				if norm.is_ok() {
					app.config.tabs.push(norm.unwrap().into_os_string().into_string().unwrap());
					app.tab_selected = app.config.tabs.len() - 1;
					spawn_scan_thread(Scanning::One(app.tab_selected));
				}
			}
		},
		DeleteCurrentTab => {
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
		ReloadCurrentTab => {
			let app = get_app();
			if app.tab_selected < app.config.tabs.len() {
				spawn_scan_thread(Scanning::One(app.tab_selected));
			}
		},
	}
	Ok(false)
}