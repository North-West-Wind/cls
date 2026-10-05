use std::{collections::{HashMap, HashSet}, eprintln, format, fs::read_dir, path::Path, println, sync::{Arc, atomic::{AtomicBool, AtomicU16, Ordering}}, thread, time::Duration, vec};

use fuzzy_matcher::{FuzzyMatcher, skim::SkimMatcherV2};
use nng::{Protocol, Socket};
use parking_lot::{Mutex, RwLock};
use rayon::iter::{IntoParallelRefIterator, ParallelBridge, ParallelExtend, ParallelIterator};
use ringbuf::{HeapCons, traits::Consumer};
use uuid::Uuid;

use crate::{common::{base::{dialog::Dialog, wave::Wave}, config::{self, SoundboardConfig}, constant::{ADDRESS_COMMS, ADDRESS_EVENT, APP_NAME}, log, socket::{ClientToServer, ServerToClient, decode_c2s, encode_c2s, encode_s2c}}, server::{audio::create_audio_player, keys::KeyCombo, pulseaudio::{load_null_sink, loopback, unload_module}, server_ext::{dialog::ServerDialog, file::ServerFile, wave::ServerWave}}};

mod audio;
mod keys;
mod pulseaudio;
mod server_ext;

pub(self) struct ServerState {
	config: SoundboardConfig,
	running: bool,

	// Audio settings
	no_pacat: bool,
	cpal_device: String,
	sample_rate: u32,
	pa_modules: HashMap<u8, (String, String)>, // ID -> (Name, Module num)

	// Audio data
	audio_data: HashMap<Uuid, Arc<Mutex<HeapCons<f32>>>>,
	stoppable: Vec<Arc<AtomicBool>>,
	playlist_lock: Arc<Mutex<()>>,

	// Waves and Dialogs
	waves: HashMap<u64, ServerWave>,
	dialogs: HashMap<u64, ServerDialog>,

	// Hotkey map
	file_keys: HashMap<KeyCombo, HashSet<String>>, // KeyCombo -> File path
	wave_keys: HashMap<KeyCombo, HashSet<u64>>, // KeyCombo -> Waveform UUID
	dialog_keys: HashMap<KeyCombo, HashSet<u64>>, // KeyCombo -> Dialog UUID

	// Hotkey
	stopkey: KeyCombo,
}

pub(self) type AtomicServerState = Arc<RwLock<ServerState>>;

impl ServerState {
	fn load_config(&mut self) {
		self.config = config::load();
	}

	fn apply_config(&mut self) {
		let config = &self.config;
		self.stopkey = KeyCombo::from_strings(config.stop_key.clone());

		// Clear all maps
		self.waves.clear();
		self.dialogs.clear();
		self.file_keys.clear();
		self.wave_keys.clear();
		self.dialog_keys.clear();

		// Load file hotkeys
		for (parent, map) in &config.files {
			for (name, entry) in map {
				let path = Path::new(&parent).join(name).to_str().unwrap().to_string();
				let combo = KeyCombo::from_strings(entry.keys.clone());
				if !combo.is_empty() && !combo.is_partial() {
					if let Some(list) = self.file_keys.get_mut(&combo) {
						list.insert(path.clone());
					} else {
						self.file_keys.insert(combo, HashSet::from_iter(vec![path.clone()]));
					}
				}
			}
		}

		// Load wave hotkeys and IDs
		config.waves.iter().for_each(|wave| {
			let combo = KeyCombo::from_strings(wave.keys.clone());
			if !combo.is_empty() && !combo.is_partial() {
				if let Some(list) = self.wave_keys.get_mut(&combo) {
					list.insert(wave.uid);
				} else {
					self.wave_keys.insert(combo, HashSet::from_iter(vec![wave.uid]));
				}
			}
			let wave = ServerWave::from(Wave::from(wave));
			self.waves.insert(wave.base.uid, wave);
		});

		// Load dialog hotkeys and IDs
		config.dialogs.iter().for_each(|dialog| {
			let combo = KeyCombo::from_strings(dialog.keys.clone());
			if !combo.is_empty() {
				if let Some(list) = self.dialog_keys.get_mut(&combo) {
					list.insert(dialog.uid);
				} else {
					self.dialog_keys.insert(combo, HashSet::from_iter(vec![dialog.uid]));
				}
			}
			let dialog = ServerDialog::from(Dialog::from(dialog));
			self.dialogs.insert(dialog.base.uid, dialog);
		});

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

	fn exit() {
		if let Ok(socket) = Socket::new(Protocol::Req0) {
			log::info("Exiting...");
			socket.dial(ADDRESS_COMMS).unwrap();
			let _ = socket.send(&encode_c2s(ClientToServer::Exit));
		}
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
	let server_state = Arc::new(RwLock::new(ServerState {
		config: config::load(),
		running: true,

		// Audio settings
		no_pacat,
		cpal_device,
		sample_rate: 48000,
		pa_modules: HashMap::new(),

		// Audio data
		audio_data: HashMap::new(),
		stoppable: vec![],
		playlist_lock: Arc::new(Mutex::new(())),

		// Waves and Dialogs
		waves: HashMap::new(),
		dialogs: HashMap::new(),

		// Hotkey maps
		file_keys: HashMap::new(),
		wave_keys: HashMap::new(),
		dialog_keys: HashMap::new(),
		
		// Hotkey
		stopkey: KeyCombo::default(),
	}));

	// Create sockets
	let server_comms = Socket::new(Protocol::Rep0)?;
	server_comms.listen(ADDRESS_COMMS)?;
	let server_event = Socket::new(Protocol::Pub0)?;
	server_event.listen(ADDRESS_EVENT)?;
	let server_event = Arc::new(Mutex::new(server_event));
	log::info(format!("COMMS is listening to {}", ADDRESS_COMMS));
	log::info(format!("EVENT is listening to {}", ADDRESS_EVENT));

	// ID generator for some server broadcasts
	let id = IdGen::default();
	
	// Load config
	server_state.write().apply_config();

	// Create audio player
	create_audio_player(server_state.clone());

	// Termination signal handler
	let _ = ctrlc::set_handler(move || ServerState::exit());
	log::info("Set SIGTERM handler");

	// Global Key Listener
	let combos: Mutex<HashSet<KeyCombo>> = Mutex::new(HashSet::new());
	let server_state_mki = server_state.clone();
	let server_event_mki = server_event.clone();
	let id_mki = id.clone();
	mki::bind_any_key(mki::Action::handle_kb(move |key| {
		let mut combos = combos.lock();
		// Remove unpressed
		combos.retain(|combo| combo.active());
		// Add new combinations
		let new_combos = combos.par_iter().map(|combo| combo.add_key(key)).collect::<Vec<_>>();
		combos.par_extend(new_combos.par_iter().cloned());
		combos.insert(KeyCombo::from_keyboards(vec![key]));

		let server_state_combo = server_state_mki.clone();
		combos.par_iter().for_each(|combo| {
			let server_state = server_state_combo.read();
			// File hotkey
			if let Some(files) = server_state.file_keys.get(combo) {
				files.par_iter().cloned().for_each(|path| {
					let file = ServerFile::new(path.clone(), &server_state);
					let server_event = server_event_mki.clone();
					let server_state = server_state_combo.clone();
					let id = id_mki.next();
					thread::spawn(move || {
						let _ = server_event.lock().send(&encode_s2c(ServerToClient::Playing(0, id, Path::new(&path).file_name().unwrap().to_str().unwrap().to_string())));
						let thread = file.play(&mut server_state.write());
						if let Some(thread) = thread {
							let _ = thread.join();
						}
						let _ = server_event.lock().send(&encode_s2c(ServerToClient::Stopping(id)));
					});
				});
			}

			// Wave hotkey
			if let Some(waves) = server_state.wave_keys.get(combo) {
				waves.par_iter().for_each(|uid| {
					if let Some(wave) = server_state.waves.get(uid) && !wave.playing.load(Ordering::Relaxed) {
						let wave = wave.clone();
						let server_state = server_state_combo.clone();
						let server_event = server_event_mki.clone();
						let id = id_mki.next();
						thread::spawn(move || {
							let _ = server_event.lock().send(&encode_s2c(ServerToClient::Playing(1, id, wave.base.label.clone())));
							let thread = wave.play(server_state);
							if let Some(thread) = thread {
								let _ = thread.join();
							}
							let _ = server_event.lock().send(&encode_s2c(ServerToClient::Stopping(id)));
						});
					}
				});
			}

			// Dialog hotkey
			if let Some(dialogs) = server_state.dialog_keys.get(combo) {
				dialogs.par_iter().for_each(|uid| {
					if let Some(dialog) = server_state.dialogs.get(uid) && !dialog.playing.load(Ordering::Relaxed) {
						let dialog = dialog.clone();
						let server_state = server_state_combo.clone();
						let server_event = server_event_mki.clone();
						let id = id_mki.next();
						thread::spawn(move || {
							let _ = server_event.lock().send(&encode_s2c(ServerToClient::Playing(2, id, dialog.base.label.clone())));
							let thread = dialog.play(server_state);
							if let Some(thread) = thread {
								let _ = thread.join();
							}
							let _ = server_event.lock().send(&encode_s2c(ServerToClient::Stopping(id)));
						});
					}
				});
			}

			// Stop hotkey
			if !server_state.stopkey.is_empty() && server_state.stopkey == *combo {
				drop(server_state);
				let mut server_state = server_state_combo.write();
				server_state.stoppable.drain(0..).par_bridge().for_each(|signal| signal.store(true, Ordering::Relaxed));
				// Consume all audio data
				server_state.audio_data.drain().par_bridge().for_each(|(_, cons)| {
					let mut cons = cons.lock();
					while let read = cons.skip(usize::MAX) && read > 0 {
						thread::sleep(Duration::from_millis(10));
					}
				});
			}
		});
	}));
	log::info("Set global key listener");

	// Socket listener - the main thing keeping this running
	while server_state.read().running {
		let mut msg = server_comms.recv()?;
		log::info(format!("Received message type {}", msg[0]));
		match decode_c2s(&mut msg) {
			Ok(request) => {
				use ClientToServer::*;
				use ServerToClient::*;
				match request {
					Exit => {
						let mut server_state = server_state.write();
						server_state.pa_modules.drain().for_each(|(_, (_, module_num))| { let _ = unload_module(&module_num); });
						server_state.running = false;
						msg.push_back(&encode_s2c(Success));
					},
					ClientToServer::Reload => {
						let mut server_state = server_state.write();
						server_state.load_config();
						server_state.apply_config();
						msg.push_back(&encode_s2c(Success));
					},
					PlayPath(path) => {
						msg.push_back(&encode_s2c(Success));
						let file = ServerFile::new(path.clone(), &server_state.read());
						let (server_state, server_event) = (server_state.clone(), server_event.clone());
						let id = id.next();
						thread::spawn(move || {
							let _ = server_event.lock().send(&encode_s2c(Playing(0, id, Path::new(&path).file_name().unwrap().to_str().unwrap().to_string())));
							let thread = file.play(&mut server_state.write());
							if let Some(thread) = thread {
								let _ = thread.join();
							}
							let _ = server_event.lock().send(&encode_s2c(Stopping(id)));
						});
					},
					PlayWave(uid) => {
						if let Some(wave) = server_state.read().waves.get(&uid).cloned() {
							if wave.playing.load(Ordering::Relaxed) {
								msg.push_back(&encode_s2c(Error(format!("Wave with ID {} is already playing", uid))));
							} else {
								msg.push_back(&encode_s2c(Success));
								let server_state = server_state.clone();
								let server_event = server_event.clone();
								let id = id.next();
								thread::spawn(move || {
									let _ = server_event.lock().send(&encode_s2c(Playing(1, id, wave.base.label.clone())));
									wave.forced.store(true, Ordering::Relaxed);
									if let Some(thread) = wave.play(server_state) {
										let _ = thread.join();
									}
									let _ = server_event.lock().send(&encode_s2c(Stopping(id)));
								});
							}
						} else {
							msg.push_back(&encode_s2c(Error(format!("Wave with ID {} not found", uid))));
						}
					},
					PlayDialog(uid) => {
						if let Some(dialog) = server_state.read().dialogs.get(&uid).cloned() {
							if dialog.playing.load(Ordering::Relaxed) {
								msg.push_back(&encode_s2c(Error(format!("Dialog with ID {} is already playing", uid))));
							} else {
								msg.push_back(&encode_s2c(Success));
								let server_state = server_state.clone();
								let server_event = server_event.clone();
								let id = id.next();
								thread::spawn(move || {
									let _ = server_event.lock().send(&encode_s2c(Playing(2, id, dialog.base.label.clone())));
									dialog.forced.store(true, Ordering::Relaxed);
									if let Some(thread) = dialog.play(server_state) {
										let _ = thread.join();
									}
									let _ = server_event.lock().send(&encode_s2c(Stopping(id)));
								});
							}
						} else {
							msg.push_back(&encode_s2c(Error(format!("Dialog with ID {} not found", uid))));
						}
					},
					PlaySearch(query) => {
						// Search
						let matcher = SkimMatcherV2::default();
						let result = server_state.read().config.tabs.par_iter().filter_map(|tab| {
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
								msg.push_back(&encode_s2c(Success));
								let file = ServerFile::new(path.clone(), &server_state.read());
								let (server_state, server_event) = (server_state.clone(), server_event.clone());
								let id = id.next();
								thread::spawn(move || {
									let _ = server_event.lock().send(&encode_s2c(Playing(0, id, Path::new(&path).file_name().unwrap().to_str().unwrap().to_string())));
									let thread = file.play(&mut server_state.write());
									if let Some(thread) = thread {
										let _ = thread.join();
									}
									let _ = server_event.lock().send(&encode_s2c(Stopping(id)));
								});
							},
							None => {
								msg.push_back(&encode_s2c(Error(format!("No result"))));
							}
						}
					},
					StopFiles => {
						let mut server_state = server_state.write();
						server_state.stoppable.drain(0..).par_bridge().for_each(|signal| signal.store(true, Ordering::Relaxed));
						// Consume all audio data
						server_state.audio_data.drain().par_bridge().for_each(|(_, cons)| {
							let mut cons = cons.lock();
							while let read = cons.skip(usize::MAX) && read > 0 {
								thread::sleep(Duration::from_millis(10));
							}
						});
						msg.push_back(&encode_s2c(Success));
					},
					StopWave(uid) => {
						match server_state.read().waves.get(&uid) {
							Some(wave) => {
								wave.forced.store(false, Ordering::Relaxed);
								msg.push_back(&encode_s2c(Success));
							},
							None => {
								msg.push_back(&encode_s2c(Error(format!("Wave with ID {} not found", uid))));
							}
						}
					},
					StopDialog(uid) => {
						match server_state.read().dialogs.get(&uid) {
							Some(dialog) => {
								dialog.forced.store(false, Ordering::Relaxed);
								msg.push_back(&encode_s2c(Success));
							},
							None => {
								msg.push_back(&encode_s2c(Error(format!("Dialog with ID {} not found", uid))));
							}
						}
					},
					SetLoopback(id, new) => {
						let mut server_state = server_state.write();
						match server_state.pa_modules.get(&id) {
							Some((old, module_num)) => {
								if *old == new {
									msg.push_back(&encode_s2c(Error(format!("Loopback {} is already set to {} ({})", id, old, module_num))));
								} else {
									let _ = unload_module(module_num);
									if !new.is_empty() {
										server_state.pa_modules.insert(id, (new.clone(), loopback(&new)));
									}
									msg.push_back(&encode_s2c(Success));
								}
							},
							None => {
								if !new.is_empty() {
									server_state.pa_modules.insert(id, (new.clone(), loopback(&new)));
								}
								msg.push_back(&encode_s2c(Success));
							}
						}
					},
					SetSinkVolume(volume) => {
						server_state.write().config.volume = volume;
						msg.push_back(&encode_s2c(Success));
					},
					SetFile(path, file) => {
						let pathed = Path::new(&path);
						let parent = pathed.parent().unwrap().to_str().unwrap().to_string();
						let name = pathed.file_name().unwrap().to_os_string().into_string().unwrap();
						let new_combo = KeyCombo::from_strings(file.keys.clone());

						let mut server_state = server_state.write();
						let old_combo = if let Some(files) = server_state.config.files.get_mut(&parent) {
							let old_combo = if let Some(old) = files.get_mut(&name) {
								let combo = KeyCombo::from_strings(old.keys.clone());
								*old = file;
								Some(combo)
							} else {
								files.insert(name, file);
								None
							};
							msg.push_back(&encode_s2c(Success));
							old_combo
						} else {
							msg.push_back(&encode_s2c(Error(format!("File isn't included in any tab"))));
							None
						};

						// Replace key combo
						if let Some(old_combo) = old_combo && let Some(list) = server_state.file_keys.get_mut(&old_combo) {
							list.remove(&path);
						}
						if !new_combo.is_empty() {
							if let Some(list) = server_state.file_keys.get_mut(&new_combo) {
								list.insert(path.clone());
							} else {
								server_state.file_keys.insert(new_combo, HashSet::from_iter(vec![path.clone()]));
							}
						}
					},
					SetWave(uid, wave) => {
						let new_combo = KeyCombo::from_strings(wave.keys.clone());
						let mut server_state = server_state.write();
						let old_combo = 
						if let Some(server_wave) = server_state.waves.get_mut(&uid) {
							// Replace existing wave
							let combo = KeyCombo::from_keyboards(server_wave.base.keys.clone());
							server_wave.base = Wave::from(&wave);
							Some(combo)
						} else {
							// Create new wave
							server_state.waves.insert(uid, ServerWave::from(Wave::from(&wave)));
							None
						};
						// Replace key combo
						if let Some(old_combo) = old_combo && let Some(list) = server_state.wave_keys.get_mut(&old_combo) {
							list.remove(&uid);
						}
						if !new_combo.is_empty() {
							if let Some(list) = server_state.wave_keys.get_mut(&new_combo) {
								list.insert(uid);
							} else {
								server_state.wave_keys.insert(new_combo, HashSet::from_iter(vec![uid]));
							}
						}
						msg.push_back(&encode_s2c(Success));
					},
					SetDialog(uid, dialog) => {
						let new_combo = KeyCombo::from_strings(dialog.keys.clone());
						let mut server_state = server_state.write();
						let old_combo = if let Some(server_dialog) = server_state.dialogs.get_mut(&uid) {
							// Replace existing dialog
							let combo = KeyCombo::from_keyboards(server_dialog.base.keys.clone());
							server_dialog.base = Dialog::from(&dialog);
							Some(combo)
						} else {
							// Create new dialog
							server_state.dialogs.insert(uid, ServerDialog::from(Dialog::from(&dialog)));
							None
						};
						// Replace key combo
						if let Some(old_combo) = old_combo && let Some(list) = server_state.dialog_keys.get_mut(&old_combo) {
							list.remove(&uid);
						}
						if !new_combo.is_empty() {
							if let Some(list) = server_state.dialog_keys.get_mut(&new_combo) {
								list.insert(uid);
							} else {
								server_state.dialog_keys.insert(new_combo, HashSet::from_iter(vec![uid]));
							}
						}
						msg.push_back(&encode_s2c(Success));
					},
					DeleteWave(uid) => {
						let mut server_state = server_state.write();
						if let Some(wave) = server_state.waves.remove(&uid) {
							let combo = KeyCombo::from_keyboards(wave.base.keys);
							if let Some(list) = server_state.wave_keys.get_mut(&combo) {
								list.remove(&uid);
							}
							msg.push_back(&encode_s2c(Success));
						} else {
							msg.push_back(&encode_s2c(Error(format!("Wave with UID {} not found", uid))));
						}
					},
					DeleteDialog(uid) => {
						let mut server_state = server_state.write();
						if let Some(dialog) = server_state.dialogs.remove(&uid) {
							let combo = KeyCombo::from_keyboards(dialog.base.keys);
							if let Some(list) = server_state.dialog_keys.get_mut(&combo) {
								list.remove(&uid);
							}
							msg.push_back(&encode_s2c(Success));
						} else {
							msg.push_back(&encode_s2c(Error(format!("Dialog with UID {} not found", uid))));
						}
					},
					SetStopKey(keys) => {
						server_state.write().stopkey = KeyCombo::from_strings(keys);
						msg.push_back(&encode_s2c(Success));
					},
					SetPlaylistMode(enabled) => {
						server_state.write().config.playlist_mode = enabled;
						msg.push_back(&encode_s2c(Success));
					},
				}
				let msg_type = msg[0];
				match server_comms.send(msg) {
					Ok(()) => log::info(format!("Replied with message type {}", msg_type)),
					Err((_, err)) => log::error(format!("Failed to reply: {:?}", err)),
				}
			},
			Err(err) => {
				log::error(format!("Socket error: {:?}", err));
			}
		}
	}

	log::info("Done. Goodbye!");

	Ok(())
}