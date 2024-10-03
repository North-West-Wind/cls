use std::{default::Default, vec::Vec};
use serde::{Serialize, Deserialize};

use crate::constant::APP_NAME;

#[derive(Serialize, Deserialize)]
struct SoundboardConfig {
	tabs: Vec<String>,
}

impl Default for SoundboardConfig {
	fn default() -> Self {
			Self {
				tabs: vec![]
			}
	}
}

static mut CONFIG: SoundboardConfig = SoundboardConfig {
	tabs: vec![]
};

pub fn load() -> Result<(), Box<dyn std::error::Error>> {
	unsafe { CONFIG = confy::load(APP_NAME, "config")? };
	Ok(())
}

pub fn save() -> Result<(), Box<dyn std::error::Error>> {
	unsafe {
		let mut cfg = SoundboardConfig::default();
		cfg.tabs = CONFIG.tabs.clone();
		confy::store(APP_NAME, "config", cfg)?;
	}
	Ok(())
}