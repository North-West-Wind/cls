use std::{default::Default, vec::Vec};
use serde::{Serialize, Deserialize};

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

static mut CONFIG: Option<SoundboardConfig> = Option::None;

pub fn load() -> Result<(), Box<dyn std::error::Error>> {
	let cfg: SoundboardConfig = confy::load("cls", "config")?;
	unsafe { CONFIG = Option::Some(cfg) };
	Ok(())
}

pub fn save() {
	unsafe {
		if CONFIG.is_some() {
			confy::store("cls", "cfg", CONFIG.unwrap());
		}
	}
}