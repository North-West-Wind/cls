use std::{collections::HashSet, time::SystemTime};

use config::Config;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::common::{base::{dialog::SaveableDialog, file::SaveableFile, wave::SaveableWave}, config::{get_config_path, migrate::migrate1::ConfigV1}, keyboard::AnyKey};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(default)]
pub struct ConfigV2 {
	pub version: u32,
	pub volume: u32,
	pub stop_key: HashSet<AnyKey>,
	pub loopbacks: Vec<String>,
	pub playlist_mode: bool,
	pub fast_scan: bool,
  #[serde(with = "indexmap::map::serde_seq")]
	pub files: IndexMap<String, IndexMap<String, SaveableFile>>,
	pub waves: Vec<SaveableWave>,
	pub dialogs: Vec<SaveableDialog>,
}

impl Default for ConfigV2 {
	fn default() -> Self {
		Self {
			version: 2,
			volume: 100,
			stop_key: HashSet::new(),
			loopbacks: vec![],
			playlist_mode: false,
			fast_scan: true,
			files: IndexMap::new(),
			waves: vec![],
			dialogs: vec![],
		}
	}
}

impl ConfigV2 {
	pub(super) fn read() -> ConfigV2 {
		let settings = Config::builder()
			.add_source(config::File::new(get_config_path(false).to_str().unwrap(), config::FileFormat::Json))
			.build()
			.expect("Failed to build config");
	
		settings.try_deserialize::<ConfigV2>().expect("Failed to parse config")
	}

	pub(super) fn from_v1(config: ConfigV1) -> ConfigV2 {
		let mut cfg = ConfigV2::default();
		cfg.volume = config.volume;
		cfg.stop_key = config.stop_key.clone();
		if config.loopback_default {
			cfg.loopbacks.push("@DEFAULT_SINK@".to_string());
		}
		if !config.loopback_1.is_empty() {
			cfg.loopbacks.push(config.loopback_1);
		}
		if !config.loopback_2.is_empty() {
			cfg.loopbacks.push(config.loopback_2);
		}
		cfg.playlist_mode = config.playlist_mode;

		for tab in config.tabs {
			if let Some(files) = config.files.get(&tab) {
				cfg.files.insert(tab, files.clone());
			}
		}
		
		// Fixing UIDs on waves and dialogs
		let mut now = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_millis() as u64;
		// Need to ensure uid doesn't repeat, don't use par_iter
		cfg.waves = config.waves.iter().map(|wave| {
			now += 1;
			SaveableWave {
				uid: now,
				label: wave.label.clone(),
				id: wave.id,
				keys: wave.keys.clone(),
				waves: wave.waves.clone(),
				volume: wave.volume
			}
		}).collect();
		cfg.dialogs = config.dialogs.iter().map(|dialog| {
			now += 1;
			SaveableDialog {
				uid: now,
				label: dialog.label.clone(),
				id: dialog.id,
				keys: dialog.keys.clone(),
				files: dialog.files.clone(),
				delay: dialog.delay,
				random: dialog.random,
				sequential: dialog.sequential,
				volume: dialog.volume
			}
		}).collect();

		cfg
	}
}