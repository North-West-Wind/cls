use std::{collections::{HashMap, HashSet}, path::Path, vec};

use config::Config;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::common::{base::{file::SaveableFile, wave::SingleWave}, keyboard::AnyKey};

use super::{get_config_path, migrate0::ConfigV0};

#[derive(Serialize, Deserialize, Debug, PartialEq, Default, Clone)]
pub(crate) struct SaveableWave {
	pub label: String,
	pub id: Option<u32>,
	pub keys: HashSet<AnyKey>,
	pub waves: Vec<SingleWave>,
	pub volume: u32,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Default, Clone)]
pub(crate) struct SaveableDialog {
	pub label: String,
	pub id: Option<u32>,
	pub keys: HashSet<AnyKey>,
	pub files: Vec<String>,
	pub delay: f32,
	pub random: bool,
	pub sequential: bool,
	pub volume: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(default)]
pub struct ConfigV1 {
	pub version: u32,
	pub tabs: Vec<String>,
	pub volume: u32,
	pub stop_key: HashSet<AnyKey>,
	pub loopback_default: bool,
	pub loopback_1: String,
	pub loopback_2: String,
	pub playlist_mode: bool,
	pub fast_scan: bool,
	pub files: IndexMap<String, IndexMap<String, SaveableFile>>,
	pub waves: Vec<SaveableWave>,
	pub dialogs: Vec<SaveableDialog>,
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
			fast_scan: true,
			files: IndexMap::new(),
			waves: vec![],
			dialogs: vec![],
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

		let mut entries: HashMap<String, SaveableFile> = HashMap::new();

		for (path, volume) in config.file_volume {
			let mut entry = SaveableFile::default();
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
					let mut entry = SaveableFile::default();
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
					let mut entry = SaveableFile::default();
					entry.id = Some(id);
					entries.insert(path, entry);
				}
			}
		}

		for (path, entry) in entries {
			let (parent, name) = parent_file(&path);
			match cfg.files.get_mut(&parent) {
				Some(map) => {
					map.insert(name, entry);
				},
				None => {
					let mut map = IndexMap::new();
					map.insert(name, entry);
					cfg.files.insert(parent, map);
				}
			};
			
		}

		cfg
	}
}

pub fn parent_file(str: &str) -> (String, String) {
	let path = Path::new(str);
	let parent = path.parent().unwrap().to_str().unwrap().to_string();
	let name = path.file_name().unwrap().to_str().unwrap().to_string();
	(parent, name)
}