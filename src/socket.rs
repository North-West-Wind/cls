use std::{cmp::{max, min}, collections::HashMap, io::{Error, Read, Write}, os::unix::net::{UnixListener, UnixStream}, path::{Path, PathBuf}};

use clap::ArgMatches;
use code::SocketCode;
use normpath::PathExt;

use crate::{component::block::{tabs::TabsBlock, BlockSingleton}, config::FileEntry, constant::APP_NAME, state::{acquire, acquire_running, load_app_config, notify_redraw, Scanning}, util::{fs::separate_parent_file, pulseaudio::{play_file, set_volume_percentage, stop_all}, threads::spawn_scan_thread, waveform::{play_wave, stop_all_waves}}};

pub mod code;

pub fn socket_path() -> PathBuf {
	std::env::temp_dir().join(APP_NAME).join(format!("{APP_NAME}.sock"))
}

pub fn ensure_socket() -> bool {
	if !socket_path().exists() {
		let _ = std::fs::create_dir_all(socket_path().parent().expect("Failed to get socket path parent"));
		return true;
	}
	false
}

pub fn listen_socket() -> std::io::Result<()> {
	let listener = UnixListener::bind(std::env::temp_dir().join(APP_NAME).join(format!("{APP_NAME}.sock")))?;
	for stream in listener.incoming() {
		let Ok(stream) = stream else { continue; };
		if let Err(err) = handle_stream(stream) {
			let mut app = acquire();
			if app.hidden {
				println!("Socket error: {:?}", err);
			} else {
				app.error = format!("Socket error: {:?}", err);
			}
		}
		if !*acquire_running() {
			break;
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
	let mut app = acquire();
	match code {
		Exit => {
			*acquire_running() = false;
			return Ok(true);
		},
		ReloadConfig => {
			let (config, stopkey, hotkey, rev_file_id, waves) = load_app_config();
			app.config = config;
			app.stopkey = stopkey;
			app.hotkey = hotkey;
			app.rev_file_id = rev_file_id;
			app.waves = waves;
		},
		AddTab => {
			let mut path = String::new();
			stream.read_to_string(&mut path)?;
			if !path.is_empty() {
				let Ok(norm) = Path::new(&path).normalize() else { return Ok(false); };
				let len = app.config.tabs.len();
				app.config.tabs.push(norm.into_os_string().into_string().unwrap());
				{ TabsBlock::instance().selected = len; }
				spawn_scan_thread(Scanning::One(len));
			}
		},
		DeleteTab|ReloadTab => {
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
				_ => chosen_index = TabsBlock::instance().selected
			}
			if code == DeleteTab {
				let key = app.config.tabs[chosen_index].clone();
				let len = app.config.tabs.len();
				let files = &mut app.files;
				files.remove(&key);
				app.config.tabs.remove(chosen_index);
				let mut tab_block = TabsBlock::instance();
				if tab_block.selected >= app.config.tabs.len() && app.config.tabs.len() != 0 {
					tab_block.selected = len - 2;
				}
				notify_redraw();
			} else {
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
		PlayId => {
			let mut bytes = [0; 4];
			stream.read_exact(&mut bytes)?;
			let id = u32::from_le_bytes(bytes);
			let path = app.rev_file_id.get(&id);
			if path.is_some() {
				let path = path.unwrap();
				if !path.is_empty() {
					play_file(&path);
				}
			}
		},
		PlayWaveId => {
			let mut bytes = [0; 4];
			stream.read_exact(&mut bytes)?;
			let id = u32::from_le_bytes(bytes);
			app.waves.iter()
				.find(|wave| { wave.id.is_some_and(|wave_id| wave_id == id) })
				.inspect(|wave| {
					play_wave((*wave).clone(), false);
				});
		},
		StopWaveId => {
			let mut bytes = [0; 4];
			stream.read_exact(&mut bytes)?;
			let id = u32::from_le_bytes(bytes);
			app.waves.iter()
				.find(|wave| { wave.id.is_some_and(|wave_id| wave_id == id) })
				.inspect(|wave| {
					let mut playing = wave.playing.lock().expect("Failed to lock mutex");
					*playing = false;
				});
		},
		Stop => {
			stop_all();
			stop_all_waves();
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
				let (parent, name) = separate_parent_file(file);
				let old_volume = match app.config.files.get(&parent) {
					Some(map) => match map.get(&name) {
						Some(entry) => entry.volume,
						None => 100
					},
					None => 100
				};
				let new_volume = min(200, max(0, if increment { old_volume as i16 + volume } else { volume })) as u32;
				if new_volume != old_volume {
					match app.config.files.get_mut(&parent) {
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