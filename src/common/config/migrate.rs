use config::Config;
use migrate0::ConfigV0;
use migrate1::ConfigV1;
use serde::Deserialize;

use crate::common::{config::migrate::migrate2::ConfigV2, constant::CONFIG_VERSION};

use super::get_config_path;

mod migrate0;
mod migrate1;
mod migrate2;

pub type SoundboardConfig = migrate2::ConfigV2;

#[derive(Deserialize, Default)]
#[serde(default)]
struct VersoinCheckConfig {
	version: u32,
}

pub(super) fn migrate_config() -> SoundboardConfig {
	let path = get_config_path(false);
	if path.exists() {
		let version = read_version();
		match version {
			0 => ConfigV2::from_v1(ConfigV1::from_v0(ConfigV0::read())),
			1 => ConfigV2::from_v1(ConfigV1::read()),
			CONFIG_VERSION => SoundboardConfig::read(),
			_ => SoundboardConfig::default()
		}
	} else {
		let path = get_config_path(true);
		if path.exists() {
			// old toml config
			ConfigV2::from_v1(ConfigV1::from_v0(ConfigV0::read()))
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