use std::{collections::{HashMap, HashSet}, path::Path, sync::{Arc, Condvar, LazyLock, Mutex, MutexGuard, OnceLock}};

use linked_hash_map::LinkedHashMap;
use mki::Keyboard;
use ratatui::{style::{Color, Style}, widgets::BorderType};
use uuid::Uuid;

use crate::{component::block::{BlockNavigation, dialogs::DialogBlock, files::FilesBlock, waves::WavesBlock}, config::{SoundboardConfig, load}, util::{dialog::Dialog, global_input::string_to_keyboard, pulseaudio::unload_module, waveform::Waveform}};

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum SelectionLayer {
	Block,
	Content
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum Scanning {
	None,
	All,
	One(usize)
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum MainOpened {
	File,
	Wave,
	Dialog,
	Log
}

impl MainOpened {
	pub fn id(&self, fallback: u8) -> u8 {
		match self {
			MainOpened::File => FilesBlock::ID,
			MainOpened::Wave => WavesBlock::ID,
			MainOpened::Dialog => DialogBlock::ID,
			_ => fallback
		}
	}
}

pub struct App {
	// config
	pub config: SoundboardConfig,
	pub hotkey: HashMap<String, Vec<Keyboard>>,
	pub stopkey: Vec<Keyboard>,
	// states
	pub error: String,
	pub error_important: bool,
	pub socket_holder: bool,
	pub hidden: bool,
	pub edit: bool,
	// render states: root
	pub block_selected: u8,
	pub selection_layer: SelectionLayer,
	pub settings_opened: bool,
	pub main_opened: MainOpened,
	// pulseaudio
	pub module_null_sink: String,
	pub module_loopback_default: String,
	pub module_loopback_1: String,
	pub module_loopback_2: String,
	// render states: files
	pub files: HashMap<String, Vec<(String, String)>>,
	pub scanning: Scanning,
	pub file_ids: HashMap<u32, String>,
	// render states: playing
	pub playing_file: LinkedHashMap<Uuid, (u32, String)>,
	pub playing_wave: LinkedHashMap<Uuid, String>,
	// waves
	pub waves: Vec<Waveform>,
	// dialog
	pub dialogs: Vec<Dialog>,
}

impl App {
	pub fn borders(&self, id: u8) -> (BorderType, Style) {
		let style = Style::default().fg(
			if self.block_selected == id {
				Color::White
			} else {
				Color::DarkGray
			}
		);
		let border_type = if self.block_selected == id {
			if self.selection_layer == SelectionLayer::Content {
				BorderType::Double
			} else {
				BorderType::Thick
			}
		} else {
			BorderType::Rounded
		};
		(border_type, style)
	}

	pub fn unload_modules(&self) {
		unload_module(&self.module_loopback_default).ok();
		unload_module(&self.module_loopback_1).ok();
		unload_module(&self.module_loopback_2).ok();
		unload_module(&self.module_null_sink).ok();
	}
}

fn key_strings_to_keyboards(keys: &HashSet<String>) -> Vec<Keyboard> {
	let mut keyboard = vec![];
	keys.iter().for_each(|key| {
		let result = string_to_keyboard(key);
		result.inspect(|result| {
			keyboard.push(*result);
		});
	});
	if keyboard.len() != keys.len() {
		keyboard.clear();
	}
	keyboard
}

pub fn load_app_config() -> (SoundboardConfig, Vec<Keyboard>, HashMap<String, Vec<Keyboard>>, HashMap<u32, String>, Vec<Waveform>, Vec<Dialog>) {
	let config = load();
	let mut stopkey = vec![];
	if config.stop_key.len() > 0 {
		config.stop_key.iter().for_each(|key| {
			let result = string_to_keyboard(key);
			result.inspect(|result| {
				stopkey.push(*result);
			});
		});
		if stopkey.len() != config.stop_key.len() {
			stopkey.clear();
		}
	}
	let mut hotkey = HashMap::new();
	let mut file_ids = HashMap::new();
	for (parent, map) in config.files.iter() {
		for (name, entry) in map {
			let path = Path::new(parent).join(name).to_str().unwrap().to_string();
			let keyboard = key_strings_to_keyboards(&entry.keys);
			if keyboard.len() > 0 && keyboard.len() == entry.keys.len() {
				hotkey.insert(path.clone(), keyboard);
			}

			entry.id.inspect(|id| {
				file_ids.insert(*id, path);
			});
		}
	}
	let mut waves = vec![];
	for wave in config.waves.iter() {
		let keyboard = key_strings_to_keyboards(&wave.keys);
		waves.push(Waveform {
			label: wave.label.clone(),
			id: wave.id,
			keys: keyboard,
			waves: wave.waves.clone(),
			volume: wave.volume,
			playing: Arc::new(Mutex::new((false, false)))
		});
	}
	let mut dialogs = vec![];
	for dialog in config.dialogs.iter() {
		let keyboard = key_strings_to_keyboards(&dialog.keys);
		dialogs.push(Dialog {
			label: dialog.label.clone(),
			id: dialog.id,
			keys: keyboard,
			files: dialog.files.clone(),
			delay: dialog.delay,
			random: dialog.random,
			play_next: 0,
			playing: Arc::new(Mutex::new((false, false)))
		});
	}

	(config, stopkey, hotkey, file_ids, waves, dialogs)
}

fn static_app(hidden: bool, edit: bool) -> &'static Mutex<App> {
	static APP: OnceLock<Mutex<App>> = OnceLock::new();
	APP.get_or_init(|| {
		let (config, stopkey, hotkey, file_ids, waves, dialogs) = load_app_config();
		let app = App {
			// config
			config,
			hotkey,
			stopkey,
			// states
			error: String::new(),
			error_important: false,
			socket_holder: false,
			hidden,
			edit,
			// render states: root
			block_selected: 0,
			selection_layer: SelectionLayer::Block,
			settings_opened: false,
			main_opened: MainOpened::File,
			// pulseaudio
			module_null_sink: String::new(),
			module_loopback_default: String::new(),
			module_loopback_1: String::new(),
			module_loopback_2: String::new(),
			// render states: files
			files: HashMap::new(),
			scanning: Scanning::None,
			file_ids,
			// render states: playing
			playing_file: LinkedHashMap::new(),
			playing_wave: LinkedHashMap::new(),
			// waves
			waves,
			// dialogs
			dialogs,
		};
		Mutex::new(app)
	})
}

pub fn init_app(hidden: bool, edit: bool) -> MutexGuard<'static, App> {
	static_app(hidden, edit).lock().unwrap()
}

pub fn acquire() -> MutexGuard<'static, App> {
	let app = static_app(false, false).lock().unwrap();
	//println!("acquire: {}", Backtrace::capture());
	app
}

static REDRAW: LazyLock<(Mutex<bool>, Condvar)> = LazyLock::new(|| (Mutex::new(true), Condvar::new()));

pub fn notify_redraw() {
	let (lock, cvar) = &*REDRAW;
	let mut shared = lock.lock().expect("Failed to get shared mutex");
	*shared = true;
	cvar.notify_all();
}

pub fn wait_redraw() {
	let (lock, cvar) = &*REDRAW;
	let mut shared = lock.lock().expect("Failed to get shared mutex");
	// Wait for redraw notice
	while !(*shared) {
		shared = cvar.wait(shared).expect("Failed to get shared mutex");
	}
	*shared = false;
}

pub fn acquire_running() -> MutexGuard<'static, bool> {
	static RUNNING: LazyLock<Mutex<bool>> = LazyLock::new(|| Mutex::new(true));
	RUNNING.lock().unwrap()
}

pub fn acquire_playlist_lock() -> MutexGuard<'static, ()> {
	static PLAYLIST_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));
	PLAYLIST_LOCK.lock().unwrap()
}