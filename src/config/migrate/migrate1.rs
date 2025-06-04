use std::collections::{HashMap, HashSet};

use config::Config;
use serde::{Deserialize, Serialize};

use crate::util::{fs::separate_parent_file, waveform::Wave};

use super::{get_config_path, migrate0::ConfigV0};

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct FileEntry {
	pub volume: u32,
	pub keys: HashSet<String>,
	pub id: Option<u32>,
}

impl Default for FileEntry {
	fn default() -> Self {
		Self {
			volume: 100,
			keys: HashSet::new(),
			id: Option::None,
		}
	}
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Default)]
pub struct WaveformEntry {
	pub label: String,
	pub id: Option<u32>,
	pub keys: HashSet<String>,
	pub waves: Vec<Wave>,
	pub volume: u32,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct ConfigV1 {
	pub version: u32,
	pub tabs: Vec<String>,
	pub volume: u32,
	pub stop_key: HashSet<String>,
	pub loopback_default: bool,
	pub loopback_1: String,
	pub loopback_2: String,
	pub playlist_mode: bool,
	pub files: HashMap<String, HashMap<String, FileEntry>>,
	pub waves: Vec<WaveformEntry>,
}

impl Default for ConfigV1 {
	fn default() -> Self {
		Self {
			version: 1,
			tabs: vec![],
			volume: 100,
			stop_key: HashSet::new(),
			loopback_default: true,
			loopback_1: String::new(),
			loopback_2: String::new(),
			playlist_mode: false,
			files: HashMap::new(),
			waves: vec![]
		}
	}
}

impl ConfigV1 {
	pub(super) fn read() -> ConfigV1 {
		let settings = Config::builder()
			.add_source(config::File::new(get_config_path(false).to_str().unwrap(), config::FileFormat::Json))
			.build()
			.expect("Failed to build config");
	
		settings.try_deserialize::<ConfigV1>().expect("Failed to parse config")
	}

	pub(super) fn from_v0(config: ConfigV0) -> ConfigV1 {
		let mut cfg = ConfigV1::default();
		cfg.tabs = config.tabs;
		cfg.volume = config.volume;
		cfg.stop_key = HashSet::from_iter(config.stop_key.into_iter());
		cfg.loopback_default = true;
		cfg.loopback_1 = config.loopback_1;
		cfg.loopback_2 = config.loopback_2;
		cfg.playlist_mode = config.playlist_mode;

		let mut entries: HashMap<String, FileEntry> = HashMap::new();

		for (path, volume) in config.file_volume {
			let mut entry = FileEntry::default();
			entry.volume = volume;
			entries.insert(path, entry);
		}

		for (path, keys) in config.file_key {
			match entries.get_mut(&path) {
				Some(en) => {
					for key in keys {
						en.keys.insert(key);
					}
				},
				None => {
					let mut entry = FileEntry::default();
					entry.keys = HashSet::from_iter(keys.into_iter());
					entries.insert(path, entry);
				}
			}
		}

		for (path, id) in config.file_id {
			match entries.get_mut(&path) {
				Some(en) => {
					en.id = Some(id);
				},
				None => {
					let mut entry = FileEntry::default();
					entry.id = Some(id);
					entries.insert(path, entry);
				}
			}
		}

		for (path, entry) in entries {
			let (parent, name) = separate_parent_file(path);
			match cfg.files.get_mut(&parent) {
				Some(map) => {
					map.insert(name, entry);
				},
				None => {
					let mut map = HashMap::new();
					map.insert(name, entry);
					cfg.files.insert(parent, map);
				}
			};
			
		}

		cfg
	}

	pub fn get_file_entry(&self, path: String) -> Option<&FileEntry> {
		let (parent, name) = separate_parent_file(path);
		match self.files.get(&parent) {
			Some(map) => match map.get(&name) {
				Some(entry) => Some(entry),
				None => None
			},
			None => None
		}
	}

	pub fn get_file_entry_mut(&mut self, path: String) -> Option<&mut FileEntry> {
		let (parent, name) = separate_parent_file(path);
		match self.files.get_mut(&parent) {
			Some(map) => match map.get_mut(&name) {
				Some(entry) => Some(entry),
				None => None
			},
			None => None
		}
	}

	pub fn insert_file_entry(&mut self, path: String, entry: FileEntry) {
		let (parent, name) = separate_parent_file(path);
		match self.files.get_mut(&parent) {
			Some(map) => {
				map.insert(name, entry);
			},
			None => {
				let mut map = HashMap::new();
				map.insert(name, entry);
				self.files.insert(parent, map);
			}
		}
	}

	pub fn remove_file_entry(&mut self, path: String) -> bool {
		let (parent, name) = separate_parent_file(path);
		match self.files.get_mut(&parent) {
			Some(map) => {
				let removed = map.remove(&name);
				return removed.is_some();
			},
			None => false
		}
	}
}

impl FileEntry {
	pub fn is_default(&self) -> bool {
		let def = Self::default();
		return self == &def;
	}
}