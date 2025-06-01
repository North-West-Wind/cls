use config::Config;
use migrate0::ConfigV0;
use migrate1::ConfigV1;
use serde::Deserialize;

use crate::constant::CONFIG_VERSION;

use super::get_config_path;

pub use migrate1::FileEntry;

mod migrate0;
mod migrate1;

pub type SoundboardConfig = migrate1::ConfigV1;

#[derive(Deserialize)]
struct VersoinCheckConfig {
	version: u32,
}

pub(super) fn migrate_config() -> SoundboardConfig {
	let path = get_config_path(false);
	if path.exists() {
		let version = read_version();
		match version {
			0 => migrate_v0(), // should not be possible, but i'm putting it here anyway
			CONFIG_VERSION => SoundboardConfig::read(),
			_ => SoundboardConfig::default()
		}
	} else {
		let path = get_config_path(true);
		if path.exists() {
			// old toml config
			migrate_v0()
		} else {
			// no config file
			SoundboardConfig::default()
		}
	}
}

fn read_version() -> u32 {
	let settings = Config::builder()
		.add_source(config::File::new(get_config_path(false).to_str().unwrap(), config::FileFormat::Json))
		.set_default("version", 1).expect("Failed to set default version for config")
		.build()
		.expect("Failed to build config");

	settings.try_deserialize::<VersoinCheckConfig>().expect("Failed to parse config").version
}

fn migrate_v0() -> ConfigV1 {
	ConfigV1::from_v0(ConfigV0::read())
}