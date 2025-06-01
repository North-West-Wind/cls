use std::{cmp::{max, min}, collections::HashMap, io::{Error, Read, Write}, os::unix::net::{UnixListener, UnixStream}, path::{Path, PathBuf}};

use clap::ArgMatches;
use code::SocketCode;
use normpath::PathExt;

use crate::{config::FileEntry, constant::APP_NAME, state::{config_mut, get_app, get_mut_app, load_app_config, Scanning}, util::{fs::separate_parent_file, notify_redraw, pulseaudio::{play_file, set_volume_percentage, stop_all}, threads::spawn_scan_thread}};

pub mod code;

pub fn socket_path() -> PathBuf {
	std::env::temp_dir().join(APP_NAME).join(format!("{APP_NAME}.sock"))
}

pub fn ensure_socket() {
	if !socket_path().exists() {
		let _ = std::fs::create_dir_all(socket_path().parent().expect("Failed to get socket path parent"));
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
				if !result.is_err_and(|err| {
					if app.hidden {
						println!("Socket error: {:?}", err);
					} else {
						app.error = format!("Socket error: {:?}", err);
					}
					true
				}) {
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
			let app = get_mut_app();
			let (config, stopkey, hotkey, rev_file_id) = load_app_config();
			app.config = config;
			app.stopkey = stopkey;
			app.hotkey = hotkey;
			app.rev_file_id = rev_file_id;
		},
		AddTab => {
			let mut path = String::new();
			stream.read_to_string(&mut path)?;
			if !path.is_empty() {
				let app = get_mut_app();
				let norm = Path::new(&path).normalize();
				norm.and_then(|norm| {
					let config = config_mut();
					config.tabs.push(norm.into_os_string().into_string().unwrap());
					app.set_tab_selected(config.tabs.len() - 1);
					spawn_scan_thread(Scanning::One(app.tab_selected()));
					Ok(())
				});
			}
		},
		DeleteTab|ReloadTab => {
			let app = get_mut_app();
			let config = config_mut();
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
					let index = config.tabs.iter().position(|tab| *tab == path);
					if index.is_none() {
						return Ok(false);
					}
					chosen_index = index.unwrap();
				},
				3 => {
					let mut name = String::new();
					stream.read_to_string(&mut name)?;
					let index = config.tabs.iter().position(|tab| Path::new(tab).file_name().unwrap().to_os_string().into_string().unwrap() == name);
					if index.is_none() {
						return Ok(false);
					}
					chosen_index = index.unwrap();
				},
				_ => chosen_index = app.tab_selected()
			}
			if code == DeleteTab {
				let files = &mut app.files;
				files.remove(&config.tabs[chosen_index]);
				config.tabs.remove(chosen_index);
				if app.tab_selected() >= config.tabs.len() && config.tabs.len() != 0 {
					app.set_tab_selected(config.tabs.len() - 1);
				}
				notify_redraw();
			} else {
				if chosen_index < config.tabs.len() {
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
		PlayId => {
			let mut bytes = [0; 4];
			stream.read_exact(&mut bytes)?;
			let id = u32::from_le_bytes(bytes);
			let app = get_app();
			let path = app.rev_file_id.get(&id);
			if path.is_some() {
				let path = path.unwrap();
				if !path.is_empty() {
					play_file(&path);
				}
			}
		},
		Stop => {
			stop_all();
		},
		SetVolume => {
			let mut args = [0; 4];
			stream.read_exact(&mut args)?;
			let first_two = args[0..2].try_into().expect("Failed to read volume bytes");
			// this should never fail, right?
			let volume = i16::from_le_bytes(first_two);
			let increment = args[2] == 1;
			let has_file = args[3] == 1;

			if !has_file {
				let config = config_mut();
				let old_volume = config.volume as i16;
				let new_volume = min(200, max(0, if increment { old_volume + volume } else { volume }));
				if new_volume != old_volume {
					set_volume_percentage(new_volume as u32);
					config.volume = new_volume as u32;
				}
			} else {
				let mut file = String::new();
				stream.read_to_string(&mut file)?;
				if file.is_empty() {
					return Ok(false);
				}
				let config = config_mut();
				let (parent, name) = separate_parent_file(file);
				let old_volume = match config.files.get(&parent) {
					Some(map) => match map.get(&name) {
						Some(entry) => entry.volume,
						None => 100
					},
					None => 100
				};
				let new_volume = min(200, max(0, if increment { old_volume as i16 + volume } else { volume })) as u32;
				if new_volume != old_volume {
					match config.files.get_mut(&parent) {
						Some(map) => match map.get_mut(&name) {
							Some(entry) => {
								entry.volume = new_volume;
							},
							None => {
								let mut entry = FileEntry::default();
								entry.volume = new_volume;
								map.insert(name, entry);
							}
						},
						None => {
							let mut map = HashMap::new();
							let mut entry = FileEntry::default();
							entry.volume = new_volume;
							map.insert(name, entry);
						}
					};
				}
			}
			notify_redraw();
		},
	}
	Ok(false)
}