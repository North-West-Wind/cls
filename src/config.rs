use std::{collections::HashMap, default::Default, vec::Vec};
use serde::{Serialize, Deserialize};

use crate::{constant::APP_NAME, state::{get_app, get_mut_app}};

#[derive(Serialize, Deserialize, Clone)]
pub struct SoundboardConfig {
	pub tabs: Vec<String>,
	pub volume: u32,
	pub file_volume: Option<HashMap<String, usize>>
}

impl Default for SoundboardConfig {
	fn default() -> Self {
		create_config()
	}
}

pub const fn create_config() -> SoundboardConfig {
	SoundboardConfig {
		tabs: vec![],
		volume: 100,
		file_volume: Option::None
	}
}

pub fn load() -> Result<(), Box<dyn std::error::Error>> {
	let app = get_mut_app();
	let cfg: SoundboardConfig = confy::load(APP_NAME, "config")?;
	(*app).config = cfg;
	Ok(())
}

pub fn save() -> Result<(), Box<dyn std::error::Error>> {
	let app = get_app();
	confy::store(APP_NAME, "config", (*app).config.clone())?;
	Ok(())
}