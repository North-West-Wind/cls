use std::collections::HashMap;

use config::Config;
use serde::{Deserialize, Serialize};

use super::get_config_path;

#[derive(Serialize, Deserialize, Debug)]
pub struct ConfigV0 {
	pub tabs: Vec<String>,
	pub volume: u32,
	pub file_volume: HashMap<String, u32>,
	pub file_key: HashMap<String, Vec<String>>,
	pub file_id: HashMap<String, u32>,
	pub stop_key: Vec<String>,
	pub loopback_1: String,
	pub loopback_2: String,
	pub playlist_mode: bool,
}

impl Default for ConfigV0 {
	fn default() -> Self {
		Self {
			tabs: vec![],
			volume: 100,
			file_volume: HashMap::new(),
			file_key: HashMap::new(),
			file_id: HashMap::new(),
			stop_key: vec![],
			loopback_1: String::new(),
			loopback_2: String::new(),
			playlist_mode: false,
		}
	}
}

impl ConfigV0 {
	pub(super) fn read() -> ConfigV0 {
		let settings = Config::builder()
			.add_source(config::File::new(get_config_path(true).to_str().unwrap(), config::FileFormat::Toml))
			.build()
			.expect("Failed to build config");
	
		settings.try_deserialize::<ConfigV0>().expect("Failed to parse config")
	}
}