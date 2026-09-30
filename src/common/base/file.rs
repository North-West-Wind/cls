use std::collections::HashSet;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct SaveableFile {
	pub volume: u32,
	pub keys: HashSet<String>,
	pub id: Option<u32>,
}

impl Default for SaveableFile {
	fn default() -> Self {
		Self {
			volume: 100,
			keys: HashSet::new(),
			id: Option::None,
		}
	}
}