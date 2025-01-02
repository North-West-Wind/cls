use std::{collections::HashMap, path::Path, sync::{Arc, Condvar, Mutex}};

use mki::Keyboard;
use std_semaphore::Semaphore;
use uuid::Uuid;

use crate::{component::{block::{files::FilesBlock, help::HelpBlock, playing::PlayingBlock, settings::SettingsBlock, tabs::TabsBlock, volume::VolumeBlock, BlockComponent}, popup::PopupComponent}, config::{load, SoundboardConfig}, util::global_input::string_to_keyboard};

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
	pub popup: Option<PopupComponent>,
	pub settings_opened: bool,
	// pulseaudio
	pub module_nums: Vec<String>,
	// render states: files
	pub files: HashMap<String, Vec<(String, String)>>,
	pub scanning: Scanning,
	pub rev_file_id: HashMap<u32, String>,
	// render states: playing
	pub playing_file: HashMap<Uuid, String>,
	pub playing_process: HashMap<Uuid, u32>,
	pub playing_semaphore: Semaphore,
}

impl App {
	pub fn file_selected(&self) -> usize {
		self.blocks[2].file_selected().unwrap()
	}

	pub fn set_file_selected(&mut self, selected: usize) {
		self.blocks[2].set_file_selected(selected);
	}

	pub fn tab_selected(&self) -> usize {
		self.blocks[1].tab_selected().unwrap()
	}

	pub fn set_tab_selected(&mut self, selected: usize) {
		self.blocks[1].set_tab_selected(selected);
	}
}

// Global variables
static mut APP: Option<App> = Option::None;

pub fn load_app_config() -> (SoundboardConfig, Vec<Keyboard>, HashMap<String, Vec<Keyboard>>, HashMap<u32, String>) {
	let config = load();
	let mut stopkey = vec![];
	if config.stop_key.len() > 0 {
		for key in config.stop_key.clone() {
			let result = string_to_keyboard(key);
			if result.is_some() {
				stopkey.push(result.unwrap());
			} else {
				break;
			}
		}
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
			let key_len = entry.keys.len();
			for key in entry.keys.clone() {
				let result = string_to_keyboard(key);
				if result.is_some() {
					keyboard.push(result.unwrap());
				} else {
					break;
				}
			}
			if keyboard.len() > 0 && keyboard.len() == key_len {
				hotkey.insert(path.clone(), keyboard);
			}

			if entry.id.is_some() {
				rev_file_id.insert(entry.id.unwrap(), path);
			}
		}
	}
	(config, stopkey, hotkey, rev_file_id)
}

pub fn init_app(hidden: bool, edit: bool) {
	unsafe {
		let (config, stopkey, hotkey, rev_file_id) = load_app_config();
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
				BlockComponent::Volume(VolumeBlock::default()),
				BlockComponent::Tabs(TabsBlock::default()),
				BlockComponent::Files(FilesBlock::default()),
				BlockComponent::Settings(SettingsBlock::default()),
				BlockComponent::Help(HelpBlock::default()),
				BlockComponent::Playing(PlayingBlock::default()),
			],
			block_selected: 0,
			selection_layer: SelectionLayer::Block,
			popup: Option::None,
			settings_opened: false,
			// pulseaudio
			module_nums: vec![],
			// render states: files
			files: HashMap::new(),
			scanning: Scanning::None,
			rev_file_id,
			// render states: playing
			playing_file: HashMap::new(),
			playing_process: HashMap::new(),
			playing_semaphore: Semaphore::new(1),
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