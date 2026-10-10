use std::{collections::{HashMap, HashSet}, eprintln, format, fs::read_dir, io::{self, Write}, net::TcpStream, path::Path, println, sync::{Arc, atomic::{AtomicBool, AtomicU16, Ordering}, mpsc::{self, Sender}}, thread, vec};

use fuzzy_matcher::{FuzzyMatcher, skim::SkimMatcherV2};
use handy_keys::KeyboardListener;
use parking_lot::RwLock;
use rayon::iter::{IntoParallelRefIterator, ParallelBridge, ParallelExtend, ParallelIterator};
use ringbuf::HeapCons;

use crate::{common::{base::{dialog::Dialog, wave::Wave}, config::{self, SoundboardConfig}, constant::{ADDRESS_COMMS, ADDRESS_EVENT, APP_NAME}, keyboard::AnyKey, log, socket::{ClientToServer, ServerToClient, encode_c2s}}, server::{audio::create_audio_player, broadcaster::TcpBroadcaster, keys::KeyCombo, pulseaudio::{load_null_sink, loopback, unload_module}, receiver::TcpReceiver, server_ext::{dialog::ServerDialog, file::ServerFile, wave::ServerWave}}};

mod audio;
mod broadcaster;
mod keys;
mod pulseaudio;
mod server_ext;
mod receiver;

pub(self) struct State {
	config: SoundboardConfig,
	running: bool
}

pub(self) struct AudioSettings {
	no_pacat: bool,
	cpal_device: String,
	sample_rate: u32,
}

pub(self) struct AudioData {
	audio_sender: Sender<HeapCons<f32>>,
	stoppable: Vec<Arc<AtomicBool>>,
}

pub(self) struct Hotkeys {
	file_keys: HashMap<KeyCombo, HashSet<String>>, // KeyCombo -> File path
	wave_keys: HashMap<KeyCombo, HashSet<u64>>, // KeyCombo -> Waveform UUID
	dialog_keys: HashMap<KeyCombo, HashSet<u64>>, // KeyCombo -> Dialog UUID
	stopkey: KeyCombo,
}

pub(self) struct ServerState {
	state: Arc<RwLock<State>>, // Write lock should only ever be held by ServerState load_config
	audio_settings: Arc<RwLock<AudioSettings>>,
	audio_data: Arc<RwLock<AudioData>>,
	hotkeys: Arc<RwLock<Hotkeys>>, // Write lock should only ever be held by ServerState load_config
	
	pa_modules: HashMap<u8, (String, String)>, // ID -> (Name, Module num)

	// Waves and Dialogs
	waves: Arc<RwLock<HashMap<u64, ServerWave>>>,
	dialogs: Arc<RwLock<HashMap<u64, ServerDialog>>>,
}

impl ServerState {
	fn load_config(&mut self) {
		let config = config::load();
		
		// Load hotkeys
		let mut hotkeys = self.hotkeys.write();
		hotkeys.stopkey = KeyCombo::from_strings(config.stop_key.clone());

		// Clear hotkeys
		hotkeys.file_keys.clear();
		hotkeys.wave_keys.clear();
		hotkeys.dialog_keys.clear();

		// Load file hotkeys
		for (parent, map) in &config.files {
			for (name, entry) in map {
				let path = Path::new(&parent).join(name).to_str().unwrap().to_string();
				let combo = KeyCombo::from_strings(entry.keys.clone());
				if !combo.is_empty() && !combo.is_partial() {
					if let Some(list) = hotkeys.file_keys.get_mut(&combo) {
						list.insert(path.clone());
					} else {
						hotkeys.file_keys.insert(combo, HashSet::from_iter(vec![path.clone()]));
					}
				}
			}
		}

		// Load wave hotkeys and IDs
		let waves = config.waves.iter().map(|wave| {
			let combo = KeyCombo::from_strings(wave.keys.clone());
			if !combo.is_empty() {
				if combo.is_partial() {
					log::warn(format!("Parsed hotkey of wave {} is partial: {}", wave.label, combo));
				}
				if let Some(list) = hotkeys.wave_keys.get_mut(&combo) {
					list.insert(wave.uid);
				} else {
					hotkeys.wave_keys.insert(combo, HashSet::from_iter(vec![wave.uid]));
				}
			}
			let wave = ServerWave::from(Wave::from(wave));
			(wave.base.uid, wave)
		}).collect::<HashMap<_, _>>();

		// Load dialog hotkeys and IDs
		let dialogs = config.dialogs.iter().map(|dialog| {
			let combo = KeyCombo::from_strings(dialog.keys.clone());
			if !combo.is_empty() {
				if let Some(list) = hotkeys.dialog_keys.get_mut(&combo) {
					list.insert(dialog.uid);
				} else {
					hotkeys.dialog_keys.insert(combo, HashSet::from_iter(vec![dialog.uid]));
				}
			}
			let dialog = ServerDialog::from(Dialog::from(dialog));
			(dialog.base.uid, dialog)
		}).collect::<HashMap<_, _>>();

		drop(hotkeys);

		// Set waves and dialogs
		(*self.waves.write()) = waves;
		(*self.dialogs.write()) = dialogs;

		// Pulseaudio loopback reload
		self.pa_modules.drain().for_each(|(_, (_, module_num))| { let _ = unload_module(&module_num); });
		self.pa_modules.insert(0, (APP_NAME.to_string(), load_null_sink()));
		if config.loopback_default {
			self.pa_modules.insert(1, ("@DEFAULT_SINK".to_string(), loopback("@DEFAULT_SINK@")));
		}
		if !config.loopback_1.is_empty() {
			self.pa_modules.insert(2, (config.loopback_1.clone(), loopback(&config.loopback_1)));
		}
		if !config.loopback_2.is_empty() {
			self.pa_modules.insert(3, (config.loopback_2.clone(), loopback(&config.loopback_2)));
		}
	}

	fn exit() -> io::Result<()> {
		log::info("Exiting...");
		let mut stream = TcpStream::connect(ADDRESS_COMMS)?;
		stream.write_all(&encode_c2s(&ClientToServer::Exit))?;
		Ok(())
	}
}

#[derive(Default, Clone)]
struct IdGen {
	current: Arc<AtomicU16>
}

impl IdGen {
	fn next(&self) -> u16 {
		let id = self.current.load(Ordering::Relaxed);
		if id == u16::MAX {
			self.current.store(0, Ordering::Relaxed);
		} else {
			self.current.store(id + 1, Ordering::Relaxed);
		}
		id
	}
}

pub fn start_server(no_pacat: bool, cpal_device: String, no_log: bool) -> Result<(), Box<dyn std::error::Error>> {
	// Initialize logger
	if !no_log {
		log::register(|level, message| {
			use log::LogLevel::*;
			match level {
				Info|Warn => println!("{}", message),
				Error => eprintln!("{}", message)
			}
		});
	}

	log::info("Starting server...");
	let (tx, rx) = mpsc::channel();
	let mut server_state = ServerState {
		state: Arc::new(RwLock::new(State {
			config: config::load(),
			running: true
		})),
		audio_settings: Arc::new(RwLock::new(AudioSettings {
			no_pacat,
			cpal_device,
			sample_rate: 48000
		})),
		audio_data: Arc::new(RwLock::new(AudioData {
			audio_sender: tx,
			stoppable: vec![]
		})),
		hotkeys: Arc::new(RwLock::new(Hotkeys {
			file_keys: HashMap::new(),
			wave_keys: HashMap::new(),
			dialog_keys: HashMap::new(),
			stopkey: KeyCombo::default()
		})),

		pa_modules: HashMap::new(),

		// Waves and Dialogs
		waves: Arc::new(RwLock::new(HashMap::new())),
		dialogs: Arc::new(RwLock::new(HashMap::new()))
	};

	// Create sockets
	let receiver = TcpReceiver::bind(ADDRESS_COMMS)?;
	let broadcaster = TcpBroadcaster::bind(ADDRESS_EVENT)?;

	// ID generator for some server broadcasts
	let id = IdGen::default();
	
	// Load config
	server_state.load_config();

	// Create audio player
	create_audio_player(server_state.state.clone(), server_state.audio_settings.clone(), rx);

	// Termination signal handler
	let _ = ctrlc::set_handler(move || { ServerState::exit().unwrap(); });
	log::info("Set SIGTERM handler");

	// Global Key Listener
	{
		let state = server_state.state.clone();
		let audio_settings = server_state.audio_settings.clone();
		let audio_data = server_state.audio_data.clone();
		let hotkeys = server_state.hotkeys.clone();
		let waves = server_state.waves.clone();
		let dialogs = server_state.dialogs.clone();

		let broadcaster = broadcaster.clone();
		let id = id.clone();
		thread::spawn(move || {
			#[cfg(target_os = "macos")]
			{
				use handy_keys::{check_accessibility, open_accessibility_settings};
				if !check_accessibility() {
					open_accessibility_settings().unwrap();
				}
			}

			let listener = KeyboardListener::new().unwrap();
			let mut combos: HashSet<KeyCombo> = HashSet::new();
			let mut modifiers = HashSet::new();

			let mut pressed = vec![];
			let mut released = vec![];

			while state.read().running {
				if let Ok(event) = listener.recv() {
					// Isolate keys and modifiers
					pressed.clear();
					released.clear();
					if let Some(key) = event.key {
						if event.is_key_down {
							pressed.push(AnyKey::K(key));
						} else {
							released.push(AnyKey::K(key));
						}
					}
					let current_modifiers = AnyKey::split_modifiers(event.modifiers);
					modifiers.retain(|modifer| {
						if !current_modifiers.contains(modifer) {
							released.push(*modifer);
							false
						} else {
							true
						}
					});
					current_modifiers.iter().for_each(|modifier| {
						if !modifiers.contains(modifier) {
							modifiers.insert(*modifier);
							pressed.push(*modifier);
						}
					});

					// Remove unpressed
					let removed = combos.extract_if(|combo| released.iter().any(|key| combo.has_key(key))).map(|combo| (combo, false)).collect::<Vec<_>>();
					// Add new combinations
					pressed.iter().for_each(|key| {
						let new_combos = combos.par_iter().map(|combo| combo.with_new_key(*key)).collect::<Vec<_>>();
						combos.par_extend(new_combos.par_iter().cloned());
						combos.insert(KeyCombo::from_keys(vec![*key]));
					});

					let sample_rate = audio_settings.read().sample_rate;
					let hotkeys = hotkeys.read();
					let config = &state.read().config;
					combos.par_iter().map(|combo| (combo.clone(), true)).chain(removed.par_iter().cloned()).for_each(|(combo, active)| {
						// File hotkey
						if active && let Some(files) = hotkeys.file_keys.get(&combo) {
							files.par_iter().cloned().for_each(|path| {
								let pathed = Path::new(&path);
								let parent = pathed.parent().unwrap().to_str().unwrap().to_string();
								let name = pathed.file_name().unwrap().to_os_string().into_string().unwrap();
								let volume = match config.files.get(&parent) {
									Some(map) => {
										match map.get(&name) {
											Some(entry) => entry.volume as f32 / 100.0,
											None => 1.0,
										}
									},
									None => 1.0
								};
								let file = ServerFile::new(path.clone(), volume, config.playlist_mode);
								let audio_data = audio_data.clone();
								let broadcaster = broadcaster.clone();
								let id = id.next();
								thread::spawn(move || {
									broadcaster.send(ServerToClient::Playing(0, id, Path::new(&path).file_name().unwrap().to_str().unwrap().to_string()));
									let thread = file.play(sample_rate, &mut audio_data.write());
									if let Some(thread) = thread {
										let _ = thread.join();
									}
									broadcaster.send(ServerToClient::Stopping(id));
								});
							});
						}

						// Wave hotkey
						if let Some(key_waves) = hotkeys.wave_keys.get(&combo) {
							key_waves.par_iter().for_each(|uid| {
								if let Some(wave) = waves.read().get(uid) {
									if active && !wave.active.load(Ordering::Relaxed) {
										let wave = wave.clone();
										let mut audio_sender = audio_data.read().audio_sender.clone();
										let broadcaster = broadcaster.clone();
										let id = id.next();
										thread::spawn(move || {
											broadcaster.send(ServerToClient::Playing(1, id, wave.base.label.clone()));
											wave.active.store(true, Ordering::Relaxed);
											let thread = wave.play(sample_rate, &mut audio_sender);
											if let Some(thread) = thread {
												let _ = thread.join();
											}
											broadcaster.send(ServerToClient::Stopping(id));
										});
									} else if !active {
										wave.active.store(false, Ordering::Relaxed);
									}
								}
							});
						}

						// Dialog hotkey
						if let Some(key_dialogs) = hotkeys.dialog_keys.get(&combo) {
							key_dialogs.par_iter().for_each(|uid| {
								if let Some(dialog) = dialogs.read().get(uid) {
									if active && !dialog.active.load(Ordering::Relaxed) {
										let dialog = dialog.clone();
										let broadcaster = broadcaster.clone();
										let audio_data = audio_data.clone();
										let id = id.next();
										thread::spawn(move || {
											broadcaster.send(ServerToClient::Playing(2, id, dialog.base.label.clone()));
											dialog.active.store(true, Ordering::Relaxed);
											let thread = dialog.play(sample_rate, audio_data);
											if let Some(thread) = thread {
												let _ = thread.join();
											}
											broadcaster.send(ServerToClient::Stopping(id));
										});
									} else if !active {
										dialog.active.store(false, Ordering::Relaxed);
									}
								}
							});
						}

						// Stop hotkey
						if !hotkeys.stopkey.is_empty() && hotkeys.stopkey == combo && active {
							audio_data.write().stoppable.drain(0..).par_bridge().for_each(|signal| signal.store(true, Ordering::Relaxed));
						}
					});
				}
			}
		});
	}
	log::info("Set global key listener");

	// Socket listener - the main thing keeping this running
	while server_state.state.read().running {
		let request = receiver.recv()?;
		use ClientToServer::*;
		use ServerToClient::*;
		match request.msg() {
			Exit => {
				server_state.pa_modules.drain().for_each(|(_, (_, module_num))| { let _ = unload_module(&module_num); });
				server_state.state.write().running = false;
				request.reply(Success);
			},
			ClientToServer::Reload => {
				server_state.load_config();
				request.reply(Success);
			},
			PlayPath(path) => {
				request.reply(Success);
				let sample_rate = server_state.audio_settings.read().sample_rate;
				let config = &server_state.state.read().config;
				let pathed = Path::new(&path);
				let parent = pathed.parent().unwrap().to_str().unwrap().to_string();
				let name = pathed.file_name().unwrap().to_os_string().into_string().unwrap();
				let volume = match config.files.get(&parent) {
					Some(map) => {
						match map.get(&name) {
							Some(entry) => entry.volume as f32 / 100.0,
							None => 1.0,
						}
					},
					None => 1.0
				};
				let file = ServerFile::new(path.clone(), volume, config.playlist_mode);
				let name = Path::new(&path).file_name().unwrap().to_str().unwrap().to_string();
				let audio_data = server_state.audio_data.clone();
				let broadcaster = broadcaster.clone();
				let id = id.next();
				thread::spawn(move || {
					broadcaster.send(Playing(0, id, name));
					let thread = file.play(sample_rate, &mut audio_data.write());
					if let Some(thread) = thread {
						let _ = thread.join();
					}
					broadcaster.send(Stopping(id));
				});
			},
			PlayWave(uid) => {
				if let Some(wave) = server_state.waves.read().get(&uid) {
					if wave.active.load(Ordering::Relaxed) {
						request.reply(Error(format!("Wave with ID {} is already playing", uid)));
					} else {
						request.reply(Success);
						let wave = wave.clone();
						let sample_rate = server_state.audio_settings.read().sample_rate;
						let mut audio_sender = server_state.audio_data.read().audio_sender.clone();
						let broadcaster = broadcaster.clone();
						let id = id.next();
						thread::spawn(move || {
							broadcaster.send(Playing(1, id, wave.base.label.clone()));
							wave.active.store(true, Ordering::Relaxed);
							if let Some(thread) = wave.play(sample_rate, &mut audio_sender) {
								let _ = thread.join();
							}
							broadcaster.send(Stopping(id));
						});
					}
				} else {
					request.reply(Error(format!("Wave with ID {} not found", uid)));
				}
			},
			PlayDialog(uid) => {
				if let Some(dialog) = server_state.dialogs.read().get(&uid) {
					if dialog.active.load(Ordering::Relaxed) {
						request.reply(Error(format!("Dialog with ID {} is already playing", uid)));
					} else {
						request.reply(Success);
						let dialog = dialog.clone();
						let sample_rate = server_state.audio_settings.read().sample_rate;
						let audio_data = server_state.audio_data.clone();
						let broadcaster = broadcaster.clone();
						let id = id.next();
						thread::spawn(move || {
							broadcaster.send(Playing(2, id, dialog.base.label.clone()));
							dialog.active.store(true, Ordering::Relaxed);
							if let Some(thread) = dialog.play(sample_rate, audio_data) {
								let _ = thread.join();
							}
							broadcaster.send(Stopping(id));
						});
					}
				} else {
					request.reply(Error(format!("Dialog with ID {} not found", uid)));
				}
			},
			PlaySearch(query) => {
				// Search
				let matcher = SkimMatcherV2::default();
				let sample_rate = server_state.audio_settings.read().sample_rate;
				let config = &server_state.state.read().config;
				let result = config.tabs.par_iter().filter_map(|tab| {
					if let Ok(entries) = read_dir(Path::new(tab)) {
						entries.into_iter().par_bridge().filter_map(|entry| {
							if let Ok(entry) = entry {
								let path = entry.path();
								if !path.is_dir() && let Some(score) = matcher.fuzzy_match(path.file_name().unwrap().to_str().unwrap(), &query) {
									return Some((score, format!("{}", path.to_str().unwrap())))
								}
							}
							None
						}).max_by(|(a, _), (b, _)| a.cmp(b))
					} else {
						None
					}
				}).max_by(|(a, _), (b, _)| a.cmp(b));
				match result {
					Some((_, path)) => {
						request.reply(Success);
						let pathed = Path::new(&path);
						let parent = pathed.parent().unwrap().to_str().unwrap().to_string();
						let name = pathed.file_name().unwrap().to_os_string().into_string().unwrap();
						let volume = match config.files.get(&parent) {
							Some(map) => {
								match map.get(&name) {
									Some(entry) => entry.volume as f32 / 100.0,
									None => 1.0,
								}
							},
							None => 1.0
						};
						let file = ServerFile::new(path.clone(), volume, config.playlist_mode);
						let audio_data = server_state.audio_data.clone();
						let broadcaster = broadcaster.clone();
						let id = id.next();
						thread::spawn(move || {
							broadcaster.send(Playing(0, id, Path::new(&path).file_name().unwrap().to_str().unwrap().to_string()));
							let thread = file.play(sample_rate, &mut audio_data.write());
							if let Some(thread) = thread {
								let _ = thread.join();
							}
							broadcaster.send(Stopping(id));
						});
					},
					None => {
						request.reply(Error(format!("No result")));
					}
				}
			},
			StopFiles => {
				server_state.audio_data.write().stoppable.drain(0..).par_bridge().for_each(|signal| signal.store(true, Ordering::Relaxed));
				request.reply(Success);
			},
			StopWave(uid) => {
				match server_state.waves.read().get(&uid) {
					Some(wave) => {
						wave.active.store(false, Ordering::Relaxed);
						request.reply(Success);
					},
					None => {
						request.reply(Error(format!("Wave with UID {} not found", uid)));
					}
				}
			},
			StopDialog(uid) => {
				match server_state.dialogs.read().get(&uid) {
					Some(dialog) => {
						dialog.active.store(false, Ordering::Relaxed);
						request.reply(Success);
					},
					None => {
						request.reply(Error(format!("Dialog with UID {} not found", uid)));
					}
				}
			},
			SetLoopback(id, new) => {
				match server_state.pa_modules.get(&id) {
					Some((old, module_num)) => {
						if *old == *new {
							request.reply(Error(format!("Loopback {} is already set to {} ({})", id, old, module_num)));
						} else {
							let _ = unload_module(module_num);
							if !new.is_empty() {
								server_state.pa_modules.insert(*id, (new.clone(), loopback(&new)));
							}
							request.reply(Success);
						}
					},
					None => {
						if !new.is_empty() {
							server_state.pa_modules.insert(*id, (new.clone(), loopback(&new)));
						}
						request.reply(Success);
					}
				}
			},
			SetSinkVolume(volume) => {
				server_state.state.write().config.volume = *volume;
				request.reply(Success);
			},
			SetFile(path, file) => {
				let pathed = Path::new(&path);
				let parent = pathed.parent().unwrap().to_str().unwrap().to_string();
				let name = pathed.file_name().unwrap().to_os_string().into_string().unwrap();
				let new_combo = KeyCombo::from_strings(file.keys.clone());

				let old_combo = if let Some(files) = server_state.state.write().config.files.get_mut(&parent) {
					let old_combo = if let Some(old) = files.get_mut(&name) {
						let combo = KeyCombo::from_strings(old.keys.clone());
						*old = file.clone();
						Some(combo)
					} else {
						files.insert(name, file.clone());
						None
					};
					request.reply(Success);
					old_combo
				} else {
					request.reply(Error(format!("File isn't included in any tab")));
					None
				};

				// Replace key combo
				let mut hotkeys = server_state.hotkeys.write();
				if let Some(old_combo) = old_combo && let Some(list) = hotkeys.file_keys.get_mut(&old_combo) {
					list.remove(path);
				}
				if !new_combo.is_empty() {
					if let Some(list) = hotkeys.file_keys.get_mut(&new_combo) {
						list.insert(path.clone());
					} else {
						hotkeys.file_keys.insert(new_combo, HashSet::from_iter(vec![path.clone()]));
					}
				}
			},
			SetWave(uid, wave) => {
				let new_combo = KeyCombo::from_strings(wave.keys.clone());
				let old_combo = 
				if let Some(server_wave) = server_state.waves.write().get_mut(&uid) {
					// Replace existing wave
					let combo = KeyCombo::from_keys(server_wave.base.keys.clone());
					server_wave.base = Wave::from(wave);
					Some(combo)
				} else {
					// Create new wave
					server_state.waves.write().insert(*uid, ServerWave::from(Wave::from(wave)));
					None
				};
				// Replace key combo
				let mut hotkeys = server_state.hotkeys.write();
				if let Some(old_combo) = old_combo && let Some(list) = hotkeys.wave_keys.get_mut(&old_combo) {
					list.remove(uid);
				}
				if !new_combo.is_empty() {
					if let Some(list) = hotkeys.wave_keys.get_mut(&new_combo) {
						list.insert(*uid);
					} else {
						hotkeys.wave_keys.insert(new_combo, HashSet::from_iter(vec![*uid]));
					}
				}
				request.reply(Success);
			},
			SetDialog(uid, dialog) => {
				let new_combo = KeyCombo::from_strings(dialog.keys.clone());
				let old_combo = if let Some(server_dialog) = server_state.dialogs.write().get_mut(&uid) {
					// Replace existing dialog
					let combo = KeyCombo::from_keys(server_dialog.base.keys.clone());
					server_dialog.base = Dialog::from(dialog);
					Some(combo)
				} else {
					// Create new dialog
					server_state.dialogs.write().insert(*uid, ServerDialog::from(Dialog::from(dialog)));
					None
				};
				// Replace key combo
				let mut hotkeys = server_state.hotkeys.write();
				if let Some(old_combo) = old_combo && let Some(list) = hotkeys.dialog_keys.get_mut(&old_combo) {
					list.remove(&uid);
				}
				if !new_combo.is_empty() {
					if let Some(list) = hotkeys.dialog_keys.get_mut(&new_combo) {
						list.insert(*uid);
					} else {
						hotkeys.dialog_keys.insert(new_combo, HashSet::from_iter(vec![*uid]));
					}
				}
				request.reply(Success);
			},
			DeleteWave(uid) => {
				if let Some(wave) = server_state.waves.write().remove(&uid) {
					let combo = KeyCombo::from_keys(wave.base.keys);
					if let Some(list) = server_state.hotkeys.write().wave_keys.get_mut(&combo) {
						list.remove(&uid);
					}
					request.reply(Success);
				} else {
					request.reply(Error(format!("Wave with UID {} not found", uid)));
				}
			},
			DeleteDialog(uid) => {
				if let Some(dialog) = server_state.dialogs.write().remove(&uid) {
					let combo = KeyCombo::from_keys(dialog.base.keys);
					if let Some(list) = server_state.hotkeys.write().dialog_keys.get_mut(&combo) {
						list.remove(&uid);
					}
					request.reply(Success);
				} else {
					request.reply(Error(format!("Dialog with UID {} not found", uid)));
				}
			},
			SetStopKey(keys) => {
				server_state.hotkeys.write().stopkey = KeyCombo::from_strings(keys);
				request.reply(Success);
			},
			SetPlaylistMode(enabled) => {
				server_state.state.write().config.playlist_mode = *enabled;
				request.reply(Success);
			},
		}
	}

	receiver.shutdown();
	broadcaster.shutdown();

	log::info("Done. Goodbye!");

	Ok(())
}