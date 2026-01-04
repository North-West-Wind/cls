use std::{cmp::{max, min}, collections::HashMap, io::{BufReader, Error, Read, Write}, os::unix::ffi::OsStrExt, path::Path};

use clap::ArgMatches;
use code::SocketCode;
use fork::{daemon, Fork};
use interprocess::local_socket::{traits::{ListenerExt, Stream as _}, GenericFilePath, GenericNamespaced, Listener, ListenerOptions, Name, NameType, Stream, ToFsName, ToNsName};
use normpath::PathExt;

use crate::{component::block::{tabs::TabsBlock, BlockSingleton}, config::FileEntry, constant::APP_NAME, state::{acquire, acquire_running, load_app_config, notify_redraw, notify_respawn, Scanning}, util::{fs::separate_parent_file, pulseaudio::{play_file, set_volume_percentage, stop_all}, threads::spawn_scan_thread, waveform::{play_wave, stop_all_waves}}};

pub mod code;

pub fn socket_name() -> std::io::Result<Name<'static>> {
	if GenericNamespaced::is_supported() {
		format!("{APP_NAME}.sock").to_ns_name::<GenericNamespaced>()
	} else {
		format!("/tmp/{APP_NAME}.sock").to_fs_name::<GenericFilePath>()
	}
}

pub fn try_socket() -> std::io::Result<Listener> {
	let opts = ListenerOptions::new().name(socket_name()?);
	opts.create_sync()
}

pub fn listen_socket(listener: Listener) {
	for conn in listener.incoming() {
		let Ok(stream) = conn else { continue; };
		if let Err(err) = handle_stream(BufReader::new(stream)) {
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
}

pub fn send_exit() -> std::io::Result<()> {
	let mut stream = Stream::connect(socket_name()?)?;
	stream.write(&[SocketCode::Exit.to_u8()])?;
	Ok(())
}

pub fn send_socket(subcommand: (&str, &ArgMatches)) -> std::io::Result<String> {
	let code = SocketCode::from_str(subcommand.0);
	if code.is_none() {
		return Err(Error::new(std::io::ErrorKind::NotFound, "Invalid command"));
	}
	let stream = Stream::connect(socket_name()?)?;
	code.unwrap().write_to_stream(stream, subcommand.1)
}

fn send_response(stream: &mut Stream, bytes: &[u8], status: bool) -> std::io::Result<bool> {
	Ok(stream.write_all(bytes).is_ok() && status)
}

fn handle_stream(mut reader: BufReader<Stream>) -> std::io::Result<bool> {
	use SocketCode::*;
	let mut code = [0];
	reader.read_exact(&mut code)?;
	let code = SocketCode::from_u8(code[0]);
	if code.is_none() {
		return Ok(false);
	}
	let code = code.unwrap();
	let mut app = acquire();
	match code {
		Exit => {
			*acquire_running() = false;
			return send_response(reader.get_mut(), &[0], true);
		},
		Pid => {
			let mut bytes = std::process::id().to_le_bytes().to_vec();
			bytes.insert(0, 0);
			return send_response(reader.get_mut(), &bytes, true);
		},
		Attach => {
			if atty::is(atty::Stream::Stdin) && atty::is(atty::Stream::Stdout) && atty::is(atty::Stream::Stderr) {
				notify_respawn();
				return send_response(reader.get_mut(), &[0], true);
			} else {
				return send_response(reader.get_mut(), &[1], false);
			}
		},
		Detach => {
			if let Ok(forked) = daemon(true, true) {
				if let Fork::Parent(pid) = forked {
					app.forked = pid;
					*acquire_running() = false;
					notify_redraw();
					notify_respawn();
					return Ok(true); // Parent is going to exit
				} else {
					app.attached = false;
					notify_redraw();
					return send_response(reader.get_mut(), &[0], true);
				}
			} else {
				return send_response(reader.get_mut(), &[1], false);
			}
		},
		ReloadConfig => {
			let (config, stopkey, hotkey, rev_file_id, waves) = load_app_config();
			app.config = config;
			app.stopkey = stopkey;
			app.hotkey = hotkey;
			app.rev_file_id = rev_file_id;
			app.waves = waves;
			notify_redraw();
			return send_response(reader.get_mut(), &[0], true);
		},
		AddTab => {
			let mut path = String::new();
			reader.read_to_string(&mut path)?;
			if !path.is_empty() {
				let Ok(norm) = Path::new(&path).normalize() else { return send_response(reader.get_mut(), &[2], false); };
				let len = app.config.tabs.len();
				app.config.tabs.push(norm.clone().into_os_string().into_string().unwrap());
				{ TabsBlock::instance().selected = len; }
				spawn_scan_thread(Scanning::One(len));
				notify_redraw();
				let mut bytes = norm.as_os_str().as_bytes().to_vec();
				bytes.insert(0, 0);
				return send_response(reader.get_mut(), &bytes, true);
			}
			return send_response(reader.get_mut(), &[1], false);
		},
		DeleteTab|ReloadTab => {
			let mut mode = [0];
			reader.read_exact(&mut mode)?;
			let mode = mode[0];
			let chosen_index: usize;
			match mode {
				1 => {
					let mut index = [0];
					reader.read_exact(&mut index)?;
					chosen_index = index[0] as usize;
				},
				2 => {
					let mut path = String::new();
					reader.read_to_string(&mut path)?;
					let path = Path::new(&path).normalize();
					if path.is_err() {
						return send_response(reader.get_mut(), &[1], false);
					}
					let path = path.unwrap().into_os_string().into_string().unwrap();
					let index = app.config.tabs.iter().position(|tab| *tab == path);
					if index.is_none() {
						return send_response(reader.get_mut(), &[2], false);
					}
					chosen_index = index.unwrap();
				},
				3 => {
					let mut name = String::new();
					reader.read_to_string(&mut name)?;
					let index = app.config.tabs.iter().position(|tab| Path::new(tab).file_name().unwrap().to_os_string().into_string().unwrap() == name);
					if index.is_none() {
						return send_response(reader.get_mut(), &[3], false);
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
				let mut bytes = key.as_bytes().to_vec();
				bytes.insert(0, 0);
				return send_response(reader.get_mut(), &bytes, true);
			} else {
				if chosen_index < app.config.tabs.len() {
					spawn_scan_thread(Scanning::One(chosen_index));
					notify_redraw();
					let path = app.config.tabs[chosen_index].clone();
					let mut bytes = path.as_bytes().to_vec();
					bytes.insert(0, 10);
					return send_response(reader.get_mut(), &bytes, true);
				}
				return send_response(reader.get_mut(), &[4], true);
			}
		},
		Play => {
			let mut path = String::new();
			reader.read_to_string(&mut path)?;
			if !path.is_empty() {
				play_file(&path);
				notify_redraw();
				let mut bytes = path.as_bytes().to_vec();
				bytes.insert(0, 0);
				return send_response(reader.get_mut(), &bytes, true);
			}
			return send_response(reader.get_mut(), &[1], false);
		},
		PlayId => {
			let mut bytes = [0; 4];
			reader.read_exact(&mut bytes)?;
			let id = u32::from_le_bytes(bytes);
			let path = app.rev_file_id.get(&id);
			if path.is_some() {
				let path = path.unwrap();
				if !path.is_empty() {
					play_file(&path);
					notify_redraw();
					let mut bytes = path.as_bytes().to_vec();
					bytes.insert(0, 0);
					return send_response(reader.get_mut(), &bytes, true);
				}
			}
			return send_response(reader.get_mut(), &[1], false);
		},
		PlayWaveId => {
			let mut bytes = [0; 4];
			reader.read_exact(&mut bytes)?;
			let id = u32::from_le_bytes(bytes);
			let wave = app.waves.iter().find(|wave| { wave.id.is_some_and(|wave_id| wave_id == id) });
			if wave.is_some() {
				let wave = wave.unwrap();
				{ wave.playing.lock().unwrap().1 = true; }
				play_wave((*wave).clone(), false);
				notify_redraw();
				let mut bytes = wave.label.as_bytes().to_vec();
				bytes.insert(0, 0);
				return send_response(reader.get_mut(), &bytes, true);
			}
			return send_response(reader.get_mut(), &[1], false);
		},
		StopWaveId => {
			let mut bytes = [0; 4];
			reader.read_exact(&mut bytes)?;
			let id = u32::from_le_bytes(bytes);
			let wave = app.waves.iter().find(|wave| { wave.id.is_some_and(|wave_id| wave_id == id) });
			if wave.is_some() {
				let wave = wave.unwrap();
				let mut playing = wave.playing.lock().expect("Failed to lock mutex");
				playing.0 = false;
				playing.1 = false;
				notify_redraw();
				let mut bytes = wave.label.as_bytes().to_vec();
				bytes.insert(0, 10);
				return send_response(reader.get_mut(), &bytes, true);
			}
			return send_response(reader.get_mut(), &[1], false);
		},
		Stop => {
			stop_all();
			stop_all_waves();
			notify_redraw();
			return send_response(reader.get_mut(), &[0], true);
		},
		SetVolume => {
			let mut args = [0; 4];
			reader.read_exact(&mut args)?;
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
				notify_redraw();
				let [a, b, c, d] = (new_volume as u32).to_le_bytes();
				return send_response(reader.get_mut(), &[0, a, b, c, d], true);
			} else {
				let mut file = String::new();
				reader.read_to_string(&mut file)?;
				if file.is_empty() {
					return send_response(reader.get_mut(), &[1], false);
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
				notify_redraw();
				let [a, b, c, d] = new_volume.to_le_bytes();
				return send_response(reader.get_mut(), &[0, a, b, c, d], true);
			}
		},
	}
}