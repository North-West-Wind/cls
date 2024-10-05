use std::{collections::HashMap, default::Default, str::FromStr, vec::Vec};
use mki::Keyboard;
use serde::{Serialize, Deserialize};

use crate::{constant::APP_NAME, state::{get_app, get_mut_app}};

#[derive(Serialize, Deserialize, Clone)]
pub struct SoundboardConfig {
	pub tabs: Vec<String>,
	pub volume: u32,
	pub file_volume: Option<HashMap<String, usize>>,
	pub file_key: Option<HashMap<String, Vec<String>>>,
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
		file_volume: Option::None,
		file_key: Option::None,
	}
}

pub fn load() -> Result<(), Box<dyn std::error::Error>> {
	let app = get_mut_app();
	let cfg: SoundboardConfig = confy::load(APP_NAME, "config")?;
	(*app).config = cfg.clone();
	app.hotkey = Option::Some(HashMap::new());

	if cfg.file_key.is_some() {
		for (path, keys) in cfg.file_key.unwrap() {
			let mut keyboard = vec![];
			let key_len = keys.len();
			for key in keys {
				let result = Keyboard::from_str(key.as_str());
				if result.is_ok() {
					keyboard.push(result.unwrap());
				}
			}
			if keyboard.len() != key_len {
				continue;
			}
			app.hotkey.as_mut().unwrap().insert(path, keyboard);
		}
	}

	Ok(())
}

pub fn save() -> Result<(), Box<dyn std::error::Error>> {
	let app = get_app();
	confy::store(APP_NAME, "config", (*app).config.clone())?;
	Ok(())
}