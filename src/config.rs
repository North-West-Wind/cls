use std::{collections::HashMap, io::Write, path::{Path, PathBuf}};
use migrate::migrate_config;
pub use migrate::SoundboardConfig;
pub use migrate::FileEntry;

use crate::{constant::APP_NAME, util::global_input::string_to_keyboard, state::{get_app, get_mut_app}};

mod migrate;

pub(self) fn get_config_path(toml: bool) -> PathBuf {
	dirs::config_dir().unwrap().join(APP_NAME).join(if toml { "config.toml" } else { "config.json" })
}

pub fn load() -> Result<(), Box<dyn std::error::Error>> {
	let app = get_mut_app();
	let cfg: SoundboardConfig = migrate_config();
	app.hotkey = Option::Some(HashMap::new());
	app.rev_file_id = Option::Some(HashMap::new());

	for (parent, map) in cfg.files.iter() {
		for (name, entry) in map {
			let path = Path::new(parent).join(name).to_str().unwrap().to_string();
			let mut keyboard = vec![];
			let key_len = entry.keys.len();
			for key in entry.keys.clone() {
				let result = string_to_keyboard(key);
				if result.is_some() {
					keyboard.push(result.unwrap());
				} else {
					break;
				}
			}
			if keyboard.len() > 0 && keyboard.len() == key_len {
				app.hotkey.as_mut().unwrap().insert(path.clone(), keyboard);
			}

			if entry.id.is_some() {
				app.rev_file_id.as_mut().unwrap().insert(entry.id.unwrap(), path);
			}
		}
	}

	if cfg.stop_key.len() > 0 {
		let mut keyboard = vec![];
		for key in cfg.stop_key.clone() {
			let result = string_to_keyboard(key);
			if result.is_some() {
				keyboard.push(result.unwrap());
			} else {
				break;
			}
		}
		if keyboard.len() == cfg.stop_key.len() {
			app.stopkey = Option::Some(keyboard);
		}
	}
	(*app).config = Option::Some(cfg);

	Ok(())
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