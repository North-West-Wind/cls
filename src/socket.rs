use std::{cmp::{max, min}, collections::HashMap, io::{Error, Read, Write}, os::unix::net::{UnixListener, UnixStream}, path::{Path, PathBuf}};

use clap::ArgMatches;
use code::SocketCode;
use normpath::PathExt;

use crate::{config, constant::APP_NAME, state::{get_app, get_mut_app, Scanning}, util::{notify_redraw, pulseaudio::{play_file, set_volume_percentage, stop_all}, threads::spawn_scan_thread}};

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
	let app = get_mut_app();
	let listener = UnixListener::bind(std::env::temp_dir().join(APP_NAME).join(format!("{APP_NAME}.sock")))?;
	for stream in listener.incoming() {
		if !app.running {
			break;
		}
		match stream {
			Ok(stream) => {
				let result = handle_stream(stream);
				if result.is_err() {
					if app.hidden {
						println!("Socket error: {:?}", result.err().unwrap());
					} else {
						app.error = format!("Socket error: {:?}", result.err().unwrap());
					}
				} else if result.ok().unwrap() {
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
	let code = code.unwrap();
	match code {
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
					app.set_tab_selected(app.config.tabs.len() - 1);
					spawn_scan_thread(Scanning::One(app.tab_selected()));
				}
			}
		},
		DeleteTab|ReloadTab => {
			let app = get_mut_app();
			let mut mode = [0];
			stream.read_exact(&mut mode)?;
			let mode = mode[0];
			let chosen_index: usize;
			match mode {
				1 => {
					let mut index = [0];
					stream.read_exact(&mut index)?;
					chosen_index = index[0] as usize;
				},
				2 => {
					let mut path = String::new();
					stream.read_to_string(&mut path)?;
					let path = Path::new(&path).normalize();
					if path.is_err() {
						return Ok(false);
					}
					let path = path.unwrap().into_os_string().into_string().unwrap();
					let index = app.config.tabs.iter().position(|tab| *tab == path);
					if index.is_none() {
						return Ok(false);
					}
					chosen_index = index.unwrap();
				},
				3 => {
					let mut name = String::new();
					stream.read_to_string(&mut name)?;
					let index = app.config.tabs.iter().position(|tab| Path::new(tab).file_name().unwrap().to_os_string().into_string().unwrap() == name);
					if index.is_none() {
						return Ok(false);
					}
					chosen_index = index.unwrap();
				},
				_ => chosen_index = app.tab_selected()
			}
			if code == DeleteTab {
				let files = app.files.as_mut();
				if files.is_some() {
					files.unwrap().remove(&app.config.tabs[chosen_index]);
					app.config.tabs.remove(chosen_index);
					if app.tab_selected() >= app.config.tabs.len() && app.config.tabs.len() != 0 {
						app.set_tab_selected(app.config.tabs.len() - 1);
					}
					notify_redraw();
				}
			} else {
				let app = get_app();
				if chosen_index < app.config.tabs.len() {
					spawn_scan_thread(Scanning::One(chosen_index));
				}
			}
		},
		Play => {
			let mut path = String::new();
			stream.read_to_string(&mut path)?;
			if !path.is_empty() {
				play_file(&path);
			}
		},
		Stop => {
			stop_all();
		},
		SetVolume => {
			let mut args = [0; 4];
			stream.read_exact(&mut args)?;
			let first_two = args[0..2].try_into();
			// this should never fail, right?
			let volume = i16::from_le_bytes(first_two.unwrap());
			let increment = args[2] == 1;
			let has_file = args[3] == 1;

			if !has_file {
				let app = get_mut_app();
				let old_volume = app.config.volume as i16;
				let new_volume = min(200, max(0, if increment { old_volume + volume } else { volume }));
				if new_volume != old_volume {
					set_volume_percentage(new_volume as u32);
					app.config.volume = new_volume as u32;
				}
			} else {
				let mut file = String::new();
				stream.read_to_string(&mut file)?;
				if file.is_empty() {
					return Ok(false);
				}
				let app = get_mut_app();
				if app.config.file_volume.is_none() {
					app.config.file_volume = Option::Some(HashMap::new());
				}
				let map = app.config.file_volume.as_mut().unwrap();
				let old_volume = map.get(&file).unwrap_or(&100);
				let new_volume = min(200, max(0, if increment { (*old_volume) as i16 + volume } else { volume })) as usize;
				if new_volume != *old_volume {
					map.insert(file, new_volume as usize);
				}
			}
			notify_redraw();
		},
	}
	Ok(false)
}