use std::{collections::HashSet, fmt::Debug, hash::Hash};

use indexmap::IndexSet;
use mki::Keyboard;

use crate::common::keyboard::string_to_keyboard;

#[derive(PartialEq, Eq, Clone, Default)]
pub struct KeyCombo {
	keys: IndexSet<Keyboard>,
	partial: bool,
}

impl Hash for KeyCombo {
	fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
		self.keys.iter().for_each(|key| {
			key.hash(state);
		});
	}
}

impl Debug for KeyCombo {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("KeyCombo")
			.field("keys", &self.keys.iter().map(|key| key.to_string()).collect::<Vec<_>>().join(" + "))
			.field("partial", &self.partial)
			.finish()
	}
}

impl From<Vec<String>> for KeyCombo {
	fn from(keys: Vec<String>) -> Self {
		let unique: HashSet<String> = HashSet::from_iter(keys.iter().cloned());
		let keys = unique.iter().filter_map(|key| string_to_keyboard(key)).collect::<Vec<_>>();
		let mut parsed = IndexSet::from_iter(keys.iter().cloned());
		let partial = parsed.len() != unique.len();

		parsed.sort_by(|a, b| a.cmp(b));

		Self {
			keys: parsed,
			partial
		}
	}
}

impl From<Vec<Keyboard>> for KeyCombo {
	fn from(keys: Vec<Keyboard>) -> Self {
		let mut keys = IndexSet::from_iter(keys.iter().cloned());
		keys.sort_by(|a, b| a.cmp(b));

		Self {
			keys,
			partial: false
		}
	}
}

impl From<HashSet<String>> for KeyCombo {
	fn from(keys: HashSet<String>) -> Self {
		Self::from(keys.into_iter().collect::<Vec<_>>())
	}
}

impl From<HashSet<Keyboard>> for KeyCombo {
	fn from(keys: HashSet<Keyboard>) -> Self {
		Self::from(keys.into_iter().collect::<Vec<_>>())
	}
}

impl KeyCombo {
	pub fn is_empty(&self) -> bool {
		self.keys.is_empty()
	}

	pub fn is_partial(&self) -> bool {
		self.partial
	}

	pub fn add_key(&self, key: Keyboard) -> Self {
		let mut keys = self.keys.iter().map(|key| *key).collect::<Vec<_>>();
		keys.push(key);
		Self::from(keys)
	}

	pub fn active(&self) -> bool {
		self.keys.iter().all(|key| key.is_pressed())
	}
}