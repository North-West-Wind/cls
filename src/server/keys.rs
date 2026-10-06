use std::{collections::HashSet, fmt::{Debug, Display}, hash::Hash, str::FromStr};

use indexmap::IndexSet;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

use crate::common::keyboard::AnyKey;

#[derive(PartialEq, Eq, Clone, Default)]
pub struct KeyCombo {
	keys: IndexSet<AnyKey>,
	partial: bool,
}

impl Hash for KeyCombo {
	fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
		// Needs ordering, don't use par_iter
		self.keys.iter().for_each(|key| {
			key.hash(state);
		});
	}
}

impl Debug for KeyCombo {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("KeyCombo")
			.field("keys", &self.to_string())
			.field("partial", &self.partial)
			.finish()
	}
}

impl Display for KeyCombo {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.keys.par_iter().map(|key| key.to_string()).collect::<Vec<_>>().join(" + "))
	}
}

impl KeyCombo {
	pub fn from_strings<I, S>(keys: I) -> Self
	where I: IntoIterator<Item = S>, S: Into<String> {
		let unique: HashSet<String> = HashSet::from_iter(keys.into_iter().map(|s| s.into()));
		let keys = unique.par_iter().filter_map(|key| AnyKey::from_str(key).ok()).collect::<Vec<_>>();
		let mut parsed = IndexSet::from_iter(keys.iter().cloned());
		let partial = parsed.len() != unique.len();

		parsed.sort_by(|a, b| a.cmp(b));

		Self {
			keys: parsed,
			partial
		}
	}

	pub fn from_keys<I, S>(keys: I) -> Self
	where I: IntoIterator<Item = S>, S: Into<AnyKey> {
		let mut keys = IndexSet::from_iter(keys.into_iter().map(|s| s.into()));
		keys.sort_by(|a, b| a.cmp(b));

		Self {
			keys,
			partial: false
		}
	}

	pub fn is_empty(&self) -> bool {
		self.keys.is_empty()
	}

	pub fn is_partial(&self) -> bool {
		self.partial
	}

	pub fn with_new_key(&self, key: AnyKey) -> Self {
		let mut keys = self.keys.par_iter().map(|key| *key).collect::<Vec<_>>();
		keys.push(key);
		Self::from_keys(keys)
	}

	pub fn has_key(&self, key: &AnyKey) -> bool {
		self.keys.contains(key)
	}
}