use std::{collections::HashMap, path::Path, sync::{Arc, Condvar, LazyLock, Mutex, MutexGuard, OnceLock}};

use linked_hash_map::LinkedHashMap;
use mki::Keyboard;
use ratatui::{style::{Color, Style}, widgets::BorderType};
use std_semaphore::Semaphore;
use uuid::Uuid;

use crate::{config::{load, SoundboardConfig}, util::{global_input::string_to_keyboard, pulseaudio::unload_module, waveform::Waveform}};

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
	pub waves_opened: bool,
	// pulseaudio
	pub module_null_sink: String,
	pub module_loopback_default: String,
	pub module_loopback_1: String,
	pub module_loopback_2: String,
	// render states: files
	pub files: HashMap<String, Vec<(String, String)>>,
	pub scanning: Scanning,
	pub rev_file_id: HashMap<u32, String>,
	// render states: playing
	pub playing_file: LinkedHashMap<Uuid, (u32, String)>,
	pub playing_semaphore: Semaphore,
	pub playing_wave: LinkedHashMap<Uuid, String>,
	// waves
	pub waves: Vec<Waveform>,
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

pub fn load_app_config() -> (SoundboardConfig, Vec<Keyboard>, HashMap<String, Vec<Keyboard>>, HashMap<u32, String>, Vec<Waveform>) {
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
	let mut rev_file_id = HashMap::new();
	for (parent, map) in config.files.iter() {
		for (name, entry) in map {
			let path = Path::new(parent).join(name).to_str().unwrap().to_string();
			let mut keyboard = vec![];
			entry.keys.iter().for_each(|key| {
				let result = string_to_keyboard(key);
				result.inspect(|result| {
					keyboard.push(*result);
				});
			});
			let key_len = entry.keys.len();
			if keyboard.len() > 0 && keyboard.len() == key_len {
				hotkey.insert(path.clone(), keyboard);
			}

			entry.id.inspect(|id| {
				rev_file_id.insert(*id, path);
			});
		}
	}
	let mut waves = vec![];
	for wave in config.waves.iter() {
		let mut keyboard = vec![];
		wave.keys.iter().for_each(|key| {
			let result = string_to_keyboard(key);
			result.inspect(|result| {
				keyboard.push(*result);
			});
		});
		if keyboard.len() == wave.keys.len() {
			waves.push(Waveform {
				label: wave.label.clone(),
				id: wave.id,
				keys: keyboard,
				waves: wave.waves.clone(),
				volume: wave.volume,
				playing: Arc::new(Mutex::new(false))
			});
		}
	}
	(config, stopkey, hotkey, rev_file_id, waves)
}

fn static_app(hidden: bool, edit: bool) -> &'static Mutex<App> {
	static APP: OnceLock<Mutex<App>> = OnceLock::new();
	APP.get_or_init(|| {
		let (config, stopkey, hotkey, rev_file_id, waves) = load_app_config();
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
			waves_opened: false,
			// pulseaudio
			module_null_sink: String::new(),
			module_loopback_default: String::new(),
			module_loopback_1: String::new(),
			module_loopback_2: String::new(),
			// render states: files
			files: HashMap::new(),
			scanning: Scanning::None,
			rev_file_id,
			// render states: playing
			playing_file: LinkedHashMap::new(),
			playing_semaphore: Semaphore::new(1),
			playing_wave: LinkedHashMap::new(),
			// waves
			waves
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