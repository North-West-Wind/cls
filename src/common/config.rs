use std::{io::Write, path::PathBuf};
use migrate::migrate_config;
pub use migrate::SoundboardConfig;

use crate::common::constant::APP_NAME;
use crate::common::log;

mod migrate;

pub(self) fn get_config_path(toml: bool) -> PathBuf {
	dirs::config_dir().expect("Could not get config directory")
		.join(APP_NAME).join(if toml { "config.toml" } else { "config.json" })
}

pub fn load() -> SoundboardConfig {
	migrate_config()
}

pub fn save(config: &SoundboardConfig) {
	let serialized = serde_json::to_string(config).expect("Failed to serialize app config");
	let config_path = get_config_path(false);
	config_path.parent().inspect(|parent| {
		let _ = std::fs::create_dir_all(parent);
	});
	if let Ok(mut output) = std::fs::File::create(get_config_path(false).to_str().unwrap()) && output.write_all(serialized.as_bytes()).is_ok() {
		log::info("Saved config");
	} else {
		log::warn("Failed to save config");
	}
}