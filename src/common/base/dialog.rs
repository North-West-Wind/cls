use std::{collections::HashSet, time::SystemTime, vec};

use serde::{Deserialize, Serialize};

use crate::common::keyboard::AnyKey;

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct SaveableDialog {
	#[serde(default)]
	pub uid: u64,
	pub label: String,
	pub id: Option<u32>,
	pub keys: HashSet<AnyKey>,
	pub files: Vec<String>,
	pub delay: f32,
	pub random: bool,
	pub sequential: bool,
	pub volume: u32,
}

impl Default for SaveableDialog {
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