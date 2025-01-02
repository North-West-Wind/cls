use std::{io::Write, path::PathBuf};
use migrate::migrate_config;
pub use migrate::SoundboardConfig;
pub use migrate::FileEntry;

use crate::{constant::APP_NAME, state::get_app};

mod migrate;

pub(self) fn get_config_path(toml: bool) -> PathBuf {
	dirs::config_dir().unwrap().join(APP_NAME).join(if toml { "config.toml" } else { "config.json" })
}

pub fn load() -> SoundboardConfig {
	migrate_config()
}

pub fn save() -> Result<(), Box<dyn std::error::Error>> {
	let serialized = serde_json::to_string(&get_app().config);
	if serialized.is_ok() {
		let output = std::fs::File::create(get_config_path(false).to_str().unwrap());
		if output.is_ok() {
			let _ = output.unwrap().write_all(serialized.unwrap().as_bytes());
		}
	}
	Ok(())
}