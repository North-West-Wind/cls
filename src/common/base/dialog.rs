use std::{collections::HashSet, time::SystemTime, vec};

use mki::Keyboard;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use serde::{Deserialize, Serialize};

use crate::common::keyboard::{keyboard_to_string, string_to_keyboard};

#[derive(Serialize, Deserialize, Debug, PartialEq, Default, Clone)]
pub struct SaveableDialog {
	#[serde(default)]
	pub uid: u64,
	pub label: String,
	pub id: Option<u32>,
	pub keys: HashSet<String>,
	pub files: Vec<String>,
	pub delay: f32,
	pub random: bool,
	pub sequential: bool,
	pub volume: u32,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct Dialog {
	pub uid: u64,
	pub label: String,
	pub id: Option<u32>,
	pub keys: HashSet<Keyboard>,
	pub files: Vec<String>,
	pub delay: f32,
	pub random: bool,
	pub sequential: bool,
	pub volume: u32,
}

impl Default for Dialog {
	fn default() -> Self {
		Self {
			uid: SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_millis() as u64,
			label: "New Dialog".to_string(),
			id: None,
			keys: HashSet::new(),
			files: vec![],
			delay: 0.2,
			random: true,
			sequential: false,
			volume: 100,
		}
	}
}

impl From<&SaveableDialog> for Dialog {
	fn from(dialog: &SaveableDialog) -> Self {
		Self {
			uid: dialog.uid,
			label: dialog.label.clone(),
			id: dialog.id,
			keys: dialog.keys.par_iter().filter_map(|key| string_to_keyboard(key)).collect::<HashSet<_>>(),
			files: dialog.files.clone(),
			delay: dialog.delay,
			random: dialog.random,
			sequential: dialog.sequential,
			volume: dialog.volume
		}
	}
}

impl Dialog {
	pub fn to_saveable(&self) -> SaveableDialog {
		SaveableDialog {
			uid: self.uid,
			label: self.label.clone(),
			id: self.id,
			keys: self.keys.par_iter().map(|key| keyboard_to_string(*key)).collect::<HashSet<_>>(),
			files: self.files.clone(),
			delay: self.delay,
			random: self.random,
			sequential: self.sequential,
			volume: self.volume
		}
	}
}