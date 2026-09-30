use std::{collections::{HashMap, HashSet}, eprintln, format, fs::read_dir, path::Path, println, sync::Arc, thread, time::SystemTime, vec};

use fuzzy_matcher::{FuzzyMatcher, skim::SkimMatcherV2};
use nng::{Protocol, Socket};
use parking_lot::{Mutex, RwLock};
use rayon::iter::{IntoParallelRefIterator, ParallelBridge, ParallelIterator};
use symphonium::SymphoniumLoader;
use uuid::Uuid;

use crate::{common::{base::{dialog::Dialog, wave::Wave}, config::{self, SoundboardConfig}, constant::{ADDRESS_COMMS, ADDRESS_EVENT, APP_NAME}, log, socket::{ClientToServer, ServerToClient, decode_c2s, encode_c2s, encode_s2c}}, server::{audio::{PlayerType, create_audio_player}, file::{PlayableFile, play_file_auto_volume, stop_all}, keys::KeyCombo, pulseaudio::{load_null_sink, loopback, unload_module}, server_ext::{dialog::ServerDialog, wave::{PlayableWave, ServerWave}}}};

mod audio;
mod file;
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
	symphonium_loader: Mutex<SymphoniumLoader>,
	playable_files: HashMap<Uuid, PlayableFile>,
	playable_waves: HashMap<u64, Vec<PlayableWave>>,
	file_cache: HashMap<String, (Vec<f32>, SystemTime)>,
	playlist_lock: Arc<Mutex<()>>,

	// Waves and Dialogs
	waves: Arc<RwLock<HashMap<u64, ServerWave>>>,
	dialogs: Arc<RwLock<HashMap<u64, ServerDialog>>>,

	// Hotkey map
	file_keys: HashMap<KeyCombo, String>, // KeyCombo -> File path
	wave_keys: HashMap<KeyCombo, u64>, // KeyCombo -> Waveform UUID
	dialog_keys: HashMap<KeyCombo, u64>, // KeyCombo -> Dialog UUID

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
		self.stopkey = KeyCombo::from(config.stop_key.clone());

		let waves = self.waves.clone();
		let dialogs = self.dialogs.clone();
		let mut waves = waves.write();
		let mut dialogs = dialogs.write();

		// Clear all maps
		waves.clear();
		dialogs.clear();
		self.file_keys.clear();
		self.wave_keys.clear();
		self.dialog_keys.clear();

		// Load file hotkeys
		for (parent, map) in &config.files {
			for (name, entry) in map {
				let path = Path::new(&parent).join(name).to_str().unwrap().to_string();
				let combo = KeyCombo::from(entry.keys.clone());
				if !combo.is_empty() && !combo.is_partial() {
					self.file_keys.insert(combo, path.clone());
				}
			}
		}

		// Load wave hotkeys and IDs
		config.waves.iter().for_each(|wave| {
			let combo = KeyCombo::from(wave.keys.clone());
			if !combo.is_empty() && !combo.is_partial() {
				self.wave_keys.insert(combo, wave.uid);
			}
			let wave = ServerWave {
				base: Wave::from(wave),
				forced: false,
			};
			waves.insert(wave.base.uid, wave);
		});

		// Load dialog hotkeys and IDs
		config.dialogs.iter().for_each(|dialog| {
			let combo = KeyCombo::from(dialog.keys.clone());
			if !combo.is_empty() {
				self.dialog_keys.insert(combo, dialog.uid);
			}
			let dialog = ServerDialog {
				base: Dialog::from(dialog),
				forced: false,
				play_next: 0,
			};
			dialogs.insert(dialog.base.uid, dialog);
		});

		// Pulseaudio loopback reload
		self.pa_modules.drain().for_each(|(_, (_, module_num))| { let _ = unload_module(&module_num); });
		self.pa_modules.insert(0, (APP_NAME.to_string(), load_null_sink()));
		if config.loopback_default {
			self.pa_modules.insert(1, ("@DEFAULT_SINK".to_string(), loopback("@DEFAULT_SINK@")));
		}
		if !config.loopback_1.is_empty() {
			self.pa_modules.insert(1, (config.loopback_1.clone(), loopback(&config.loopback_1)));
		}
		if !config.loopback_2.is_empty() {
			self.pa_modules.insert(1, (config.loopback_2.clone(), loopback(&config.loopback_2)));
		}
	}

	fn exit() {
		if let Ok(socket) = Socket::new(Protocol::Req0) {
			log::info("Exiting...".to_string());
			socket.dial(ADDRESS_COMMS).unwrap();
			let _ = socket.send(&encode_c2s(ClientToServer::Exit));
		}
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

	log::info("Starting server...".to_string());
	let server_state = Arc::new(RwLock::new(ServerState {
		config: config::load(),
		running: true,

		// Audio settings
		no_pacat,
		cpal_device,
		sample_rate: 48000,
		pa_modules: HashMap::new(),

		// Audio data
		symphonium_loader: Mutex::new(SymphoniumLoader::new()),
		playable_files: HashMap::new(),
		playable_waves: HashMap::new(),
		file_cache: HashMap::new(),
		playlist_lock: Arc::new(Mutex::new(())),

		// Waves and Dialogs
		waves: Arc::new(RwLock::new(HashMap::new())),
		dialogs: Arc::new(RwLock::new(HashMap::new())),

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
	
	// Load config
	server_state.write().apply_config();

	// Create audio players
	create_audio_player(server_state.clone(), PlayerType::File);
	create_audio_player(server_state.clone(), PlayerType::Wave);

	// Termination signal handler
	let _ = ctrlc::set_handler(move || ServerState::exit());
	log::info("Set SIGTERM handler".to_string());

	// Global Key Listener
	let combos: Mutex<HashSet<KeyCombo>> = Mutex::new(HashSet::new());
	let server_state_mki = server_state.clone();
	let server_event_mki = server_event.clone();
	mki::bind_any_key(mki::Action::handle_kb(move |key| {
		let mut combos = combos.lock();
		if key.is_pressed() {
			// Create new ones from existing combos
			let mut new_combos = vec![];
			for combo in combos.iter() {
				let new_combo = combo.add_key(key);
				new_combos.push(new_combo);
			}
			new_combos.iter().for_each(|combo| { combos.insert(combo.clone()); });
			combos.insert(KeyCombo::from(vec![key]));
		} else {
			combos.retain(|combo| !combo.contains_key(&key));
		}
		let server_state_combo = server_state_mki.clone();
		combos.par_iter().for_each(|combo| {
			let server_state = server_state_combo.read();
			// File hotkey
			if server_state.file_keys.contains_key(combo) {
				let server_event = server_event_mki.clone();
				let path = server_state.file_keys.get(combo).unwrap().clone();
				let server_state = server_state_combo.clone();
				thread::spawn(move || {
					let uuid = Uuid::new_v4();
					let _ = server_event.lock().send(&encode_s2c(ServerToClient::Playing(0, uuid, path.clone())));
					let _ = play_file_auto_volume(server_state, path, true);
					let _ = server_event.lock().send(&encode_s2c(ServerToClient::Stopping(uuid)));
				});
			}

			// Wave hotkey
			if server_state.wave_keys.contains_key(combo) {
				let uid = *server_state.wave_keys.get(combo).unwrap();
				let waves = server_state.waves.clone();
				let server_state = server_state_combo.clone();
				let server_event = server_event_mki.clone();
				thread::spawn(move || {
					let waves = waves.read();
					let wave = waves.get(&uid).unwrap();
					let uuid = Uuid::new_v4();
					let _ = server_event.lock().send(&encode_s2c(ServerToClient::Playing(1, uuid, wave.base.label.clone())));
					wave.play(server_state, false);
					let _ = server_event.lock().send(&encode_s2c(ServerToClient::Stopping(uuid)));
				});
			}

			// Dialog hotkey
			if server_state.dialog_keys.contains_key(combo) {
				let uid = *server_state.dialog_keys.get(combo).unwrap();
				let dialogs = server_state.dialogs.clone();
				let server_state = server_state_combo.clone();
				let server_event = server_event_mki.clone();
				thread::spawn(move || {
					let dialogs = dialogs.read();
					let dialog = dialogs.get(&uid).unwrap();
					let uuid = Uuid::new_v4();
					let _ = server_event.lock().send(&encode_s2c(ServerToClient::Playing(2, uuid, dialog.base.label.clone())));
					dialog.play(server_state, false);
					let _ = server_event.lock().send(&encode_s2c(ServerToClient::Stopping(uuid)));
				});
			}

			// Stop hotkey
			if !server_state.stopkey.is_empty() && server_state.stopkey == *combo {
				stop_all(server_state_combo.clone());
			}
		});
	}));
	log::info("Set global key listener".to_string());

	// Socket listener - the main thing keeping this running
	while server_state.read().running {
		let mut msg = server_comms.recv()?;
		log::info(format!("Received message type {}", msg[0]));
		match decode_c2s(&msg) {
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
						log::info(format!("Playing {}", path));
						msg.push_back(&encode_s2c(Success));
						let (server_state, server_event) = (server_state.clone(), server_event.clone());
						thread::spawn(move || {
							let uuid = Uuid::new_v4();
							let _ = server_event.lock().send(&encode_s2c(Playing(0, uuid, path.clone())));
							let _ = play_file_auto_volume(server_state, path, true);
							let _ = server_event.lock().send(&encode_s2c(Stopping(uuid)));
						});
					},
					PlayWave(uid) => {
						if server_state.read().waves.read().contains_key(&uid) {
							msg.push_back(&encode_s2c(Success));
							let server_state = server_state.clone();
							let server_event = server_event.clone();
							thread::spawn(move || {
								let waves = { server_state.read().waves.clone() };
								if let Some(wave) = waves.write().get_mut(&uid) {
									let uuid = Uuid::new_v4();
									let _ = server_event.lock().send(&encode_s2c(Playing(1, uuid, wave.base.label.clone())));
									wave.forced = true;
									wave.play(server_state, false);
									let _ = server_event.lock().send(&encode_s2c(Stopping(uuid)));
								}
							});
						} else {
							msg.push_back(&encode_s2c(Error(format!("Wave with ID {} not found", uid))));
						}
					},
					PlayDialog(uid) => {
						if server_state.read().dialogs.read().contains_key(&uid) {
							msg.push_back(&encode_s2c(Success));
							let server_state = server_state.clone();
							let server_event = server_event.clone();
							thread::spawn(move || {
								let dialogs = { server_state.read().dialogs.clone() };
								if let Some(dialog) = dialogs.write().get_mut(&uid) {
									let uuid = Uuid::new_v4();
									let _ = server_event.lock().send(&encode_s2c(Playing(2, uuid, dialog.base.label.clone())));
									dialog.forced = true;
									dialog.play(server_state, true);
									let _ = server_event.lock().send(&encode_s2c(Stopping(uuid)));
								}
							});
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
										let path = entry.path().canonicalize().unwrap();
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
								let path = path.to_string();
								msg.push_back(&encode_s2c(Success));
								let (server_state, server_event) = (server_state.clone(), server_event.clone());
								thread::spawn(move || {
									let uuid = Uuid::new_v4();
									let _ = server_event.lock().send(&encode_s2c(Playing(0, uuid, path.clone())));
									let _ = play_file_auto_volume(server_state, path, true);
									let _ = server_event.lock().send(&encode_s2c(Stopping(uuid)));
								});
							},
							None => {
								msg.push_back(&encode_s2c(Error(format!("No result"))));
							}
						}
					},
					StopFiles => {
						stop_all(server_state.clone());
						msg.push_back(&encode_s2c(Success));
					},
					StopWave(uid) => {
						match server_state.read().waves.write().get_mut(&uid) {
							Some(wave) => {
								wave.forced = false;
								msg.push_back(&encode_s2c(Success));
							},
							None => {
								msg.push_back(&encode_s2c(Error(format!("Wave with ID {} not found", uid))));
							}
						}
					},
					StopDialog(uid) => {
						match server_state.read().dialogs.write().get_mut(&uid) {
							Some(dialog) => {
								dialog.forced = false;
								msg.push_back(&encode_s2c(Success));
							},
							None => {
								msg.push_back(&encode_s2c(Error(format!("Dialog with ID {} not found", uid))));
							}
						}
					},
				}
				match server_comms.send(msg) {
					Ok(()) => log::info("Replied".to_string()),
					Err((_, err)) => log::error(format!("Failed to reply: {:?}", err)),
				}
			},
			Err(err) => {
				log::error(format!("Socket error: {:?}", err));
			}
		}
	}

	log::info("Done. Goodbye!".to_string());

	Ok(())
}