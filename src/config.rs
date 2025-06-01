use std::{io::Write, path::PathBuf};
use migrate::migrate_config;
pub use migrate::SoundboardConfig;
pub use migrate::FileEntry;

use crate::{constant::APP_NAME, state::get_app};

mod migrate;

pub(self) fn get_config_path(toml: bool) -> PathBuf {
	dirs::config_dir().expect("Could not get config directory")
		.join(APP_NAME).join(if toml { "config.toml" } else { "config.json" })
}

pub fn load() -> SoundboardConfig {
	migrate_config()
}

pub fn save() {
	let serialized = serde_json::to_string(&get_app().config).expect("Failed to serialize app config");
	let _ = std::fs::File::create(get_config_path(false).to_str().unwrap()).is_ok_and(|mut output| {
		output.write_all(serialized.as_bytes()).is_ok()
	});
}