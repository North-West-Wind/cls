use std::{collections::HashMap, path::Path, sync::{Arc, Condvar, Mutex}};

use mki::Keyboard;
use std_semaphore::Semaphore;
use uuid::Uuid;

use crate::{component::{block::{files::FilesBlock, help::HelpBlock, playing::PlayingBlock, settings::SettingsBlock, tabs::TabsBlock, volume::VolumeBlock, waves::WavesBlock, BlockComponent, BlockNavigation}, popup::PopupComponent}, config::{load, SoundboardConfig}, util::{global_input::string_to_keyboard, pulseaudio::unload_module, waveform::Waveform}};

pub type CondvarPair = Arc<(Mutex<SharedCondvar>, Condvar)>;

pub struct SharedCondvar {
	pub redraw: bool,
}

impl Default for SharedCondvar {
	fn default() -> Self {
		Self {
			redraw: true
		}
	}
}

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
	pub running: bool,
	pub error: String,
	pub error_important: bool,
	pub pair: CondvarPair,
	pub socket_holder: bool,
	pub hidden: bool,
	pub edit: bool,
	// render states: root
	pub blocks: Vec<BlockComponent>,
	pub block_selected: u8,
	pub selection_layer: SelectionLayer,
	pub popups: Vec<PopupComponent>,
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
	pub playing_file: HashMap<Uuid, (u32, String)>,
	pub playing_semaphore: Semaphore,
	pub playing_wave: HashMap<Uuid, (u32, String)>,
	// waves
	pub waves: Vec<Waveform>,
}

impl App {
	pub fn file_selected(&self) -> usize {
		self.blocks[FilesBlock::ID as usize].file_selected().unwrap()
	}

	pub fn set_file_selected(&mut self, selected: usize) {
		self.blocks[FilesBlock::ID as usize].set_file_selected(selected);
	}

	pub fn tab_selected(&self) -> usize {
		self.blocks[TabsBlock::ID as usize].tab_selected().unwrap()
	}

	pub fn set_tab_selected(&mut self, selected: usize) {
		self.blocks[TabsBlock::ID as usize].set_tab_selected(selected);
	}

	pub fn wave_selected(&self) -> usize {
		self.blocks[WavesBlock::ID as usize].wave_selected().unwrap()
	}

	pub fn set_wave_selected(&mut self, selected: usize) {
		self.blocks[WavesBlock::ID as usize].set_wave_selected(selected);
	}

	pub fn unload_modules(&self) {
		unload_module(&self.module_loopback_default).ok();
		unload_module(&self.module_loopback_1).ok();
		unload_module(&self.module_loopback_2).ok();
		unload_module(&self.module_null_sink).ok();
	}
}

// Global variables
static mut APP: Option<App> = Option::None;

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

pub fn init_app(hidden: bool, edit: bool) {
	use BlockComponent::*;
	unsafe {
		let (config, stopkey, hotkey, rev_file_id, waves) = load_app_config();
		let app = App {
			// config
			config,
			hotkey,
			stopkey,
			// states
			running: false,
			error: String::new(),
			error_important: false,
			pair: Arc::new((Mutex::new(SharedCondvar::default()), Condvar::new())),
			socket_holder: false,
			hidden,
			edit,
			// render states: root
			blocks: vec![
				Volume(VolumeBlock::default()),
				Tabs(TabsBlock::default()),
				Files(FilesBlock::default()),
				Settings(SettingsBlock::default()),
				Help(HelpBlock::default()),
				Playing(PlayingBlock::default()),
				Waves(WavesBlock::default())
			],
			block_selected: 0,
			selection_layer: SelectionLayer::Block,
			popups: vec![],
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
			playing_file: HashMap::new(),
			playing_semaphore: Semaphore::new(1),
			playing_wave: HashMap::new(),
			// waves
			waves
		};
	
		APP = Option::Some(app);
	}
}

pub fn get_mut_app() -> &'static mut App {
	unsafe { APP.as_mut().unwrap() }
}

pub fn get_app() -> &'static App {
	unsafe { APP.as_ref().unwrap() }
}

pub fn config() -> &'static SoundboardConfig {
	&get_app().config
}

pub fn config_mut() -> &'static mut SoundboardConfig {
	&mut get_mut_app().config
}