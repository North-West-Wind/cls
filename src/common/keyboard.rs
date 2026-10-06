use std::{cmp::Ordering, error::Error, fmt::Display, str::FromStr};

use handy_keys::{Key, Modifiers};
use regex::Regex;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseAnyKeyError;

impl Display for ParseAnyKeyError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    "invalid key".fmt(f)
	}
}

impl Error for ParseAnyKeyError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AnyKey {
	M(Modifiers),
	K(Key),
}

impl FromStr for AnyKey {
	type Err = ParseAnyKeyError;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		if let Ok(key) = Key::from_str(s) {
			Ok(AnyKey::K(key))
		} else if let Ok(modifier) = Modifiers::from_str(s) {
			Ok(AnyKey::M(modifier))
		} else {
			match s.to_lowercase().as_str() {
				"leftsuper" => Ok(AnyKey::M(Modifiers::CMD_LEFT)),
				"rightsuper" => Ok(AnyKey::M(Modifiers::CMD_RIGHT)),
				"leftshift" => Ok(AnyKey::M(Modifiers::SHIFT_LEFT)),
				"rightshift" => Ok(AnyKey::M(Modifiers::SHIFT_RIGHT)),
				"leftctrl" => Ok(AnyKey::M(Modifiers::CTRL_LEFT)),
				"rightctrl" => Ok(AnyKey::M(Modifiers::CTRL_RIGHT)),
				"leftalt" => Ok(AnyKey::M(Modifiers::OPT_LEFT)),
				"rightalt" => Ok(AnyKey::M(Modifiers::OPT_RIGHT)),
				"n0" => Ok(AnyKey::K(Key::Keypad0)),
				"n1" => Ok(AnyKey::K(Key::Keypad1)),
				"n2" => Ok(AnyKey::K(Key::Keypad2)),
				"n3" => Ok(AnyKey::K(Key::Keypad3)),
				"n4" => Ok(AnyKey::K(Key::Keypad4)),
				"n5" => Ok(AnyKey::K(Key::Keypad5)),
				"n6" => Ok(AnyKey::K(Key::Keypad6)),
				"n7" => Ok(AnyKey::K(Key::Keypad7)),
				"n8" => Ok(AnyKey::K(Key::Keypad8)),
				"n9" => Ok(AnyKey::K(Key::Keypad9)),
				"n." => Ok(AnyKey::K(Key::KeypadDecimal)),
				"n+" => Ok(AnyKey::K(Key::KeypadPlus)),
				"n-" => Ok(AnyKey::K(Key::KeypadMinus)),
				"n*" => Ok(AnyKey::K(Key::KeypadMultiply)),
				"n/" => Ok(AnyKey::K(Key::KeypadDivide)),
				_ => Err(ParseAnyKeyError)
			}
		}
	}
}

impl ToString for AnyKey {
	fn to_string(&self) -> String {
		match self {
			AnyKey::K(key) => key.to_string(),
			AnyKey::M(modifiers) => modifiers.to_string()
		}
	}
}

impl PartialOrd for AnyKey {
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
		if let AnyKey::M(_) = self && let AnyKey::K(_) = other {
			Some(Ordering::Less)
		} else if let AnyKey::K(_) = self && let AnyKey::M(_) = other {
			Some(Ordering::Greater)
		} else {
			self.to_string().partial_cmp(&other.to_string())
		}
	}
}

impl Ord for AnyKey {
	fn cmp(&self, other: &Self) -> Ordering {
		if let AnyKey::M(_) = self && let AnyKey::K(_) = other {
			Ordering::Less
		} else if let AnyKey::K(_) = self && let AnyKey::M(_) = other {
			Ordering::Greater
		} else {
			self.to_string().cmp(&other.to_string())
		}
	}
}

impl AnyKey {
	pub fn split_modifiers(modifiers: Modifiers) -> Vec<AnyKey> {
		let str = modifiers.to_string();
		if str.is_empty() {
			vec![]
		} else {
			str.split('+').map(|s| AnyKey::M(Modifiers::from_str(s).unwrap())).collect()
		}
	}
}

// Custom key name ordering:
// 1. FN keys
// 2. Other keys (backspace, shift, etc.)
// 3. Letter keys
// 4. Number keys
// 5. Symbol keys
pub fn key_sorter(a: &str, b: &str) -> Ordering {
	let regex_fn = Regex::new(r"F\d").unwrap();
	let regex_a = regex_fn.is_match(a);
	let regex_b = regex_fn.is_match(b);
	if regex_a && !regex_b {
		Ordering::Less
	} else if !regex_a && regex_b {
		Ordering::Greater
	} else if regex_a && regex_b {
		a.cmp(b)
	} else {
		let single_a = a.len() == 1;
		let single_b = b.len() == 1;
		if !single_a && single_b {
			Ordering::Less
		} else if single_a && !single_b {
			Ordering::Greater
		} else if !single_a && !single_b {
			a.cmp(b)
		} else {
			let char_a = a.chars().next().expect("a is empty");
			let char_b = b.chars().next().expect("a is empty");
			let letter_a = char_a.is_alphabetic();
			let letter_b = char_b.is_alphabetic();
			if letter_a && !letter_b {
				Ordering::Less
			} else if !letter_a && letter_b {
				Ordering::Greater
			} else if letter_a && letter_b {
				a.cmp(b)
			} else {
				let digit_a = char_a.is_digit(10);
				let digit_b = char_b.is_digit(10);
				if digit_a && !digit_b {
					Ordering::Less
				} else if !digit_a && digit_b {
					Ordering::Greater
				} else {
					a.cmp(b)
				}
			}
		}
	}
}