use std::str::FromStr;

use mki::Keyboard;
use substring::Substring;

pub fn keyboard_to_string(keyboard: Keyboard) -> String {
	use Keyboard::*;
	let str = match keyboard {
		Number0 => "0",
		Number1 => "1",
		Number2 => "2",
		Number3 => "3",
		Number4 => "4",
		Number5 => "5",
		Number6 => "6",
		Number7 => "7",
		Number8 => "8",
		Number9 => "9",
		Comma => ",",
		Period => ".",
		Slash => "/",
		SemiColon => ";",
		Apostrophe => "'",
		LeftBrace => "[",
		RightBrace => "]",
		BackwardSlash => "\\",
		Grave => "`",
		Other(code) => match code {
			56 => "LeftAlt",
			100 => "RightAlt",
			125 => "LeftSuper",
			12 => "_",
			13 => "=",
			55 => "N*",
			71 => "N7",
			72 => "N8",
			73 => "N9",
			74 => "N-",
			75 => "N4",
			76 => "N5",
			77 => "N6",
			78 => "N+",
			79 => "N1",
			80 => "N2",
			81 => "N3",
			82 => "N0",
			83 => "N.",
			98 => "N/",
			_ => "",
		},
		_ => ""
	}.to_string();
	if str.is_empty() {
		return match keyboard {
			Other(code) => format!("({})", code),
			_ => format!("{:?}", keyboard)
		};
	}
	str
}

pub fn string_to_keyboard(string: &str) -> Option<Keyboard> {
	use Keyboard::Other;
	match string {
		"LeftAlt" => Some(Other(56)),
		"RightAlt" => Some(Other(100)),
		"LeftSuper" => Some(Other(125)),
		"_" => Some(Other(12)),
		"=" => Some(Other(13)),
		"N*" => Some(Other(55)),
		"N7" => Some(Other(71)),
		"N8" => Some(Other(72)),
		"N9" => Some(Other(73)),
		"N-" => Some(Other(74)),
		"N4" => Some(Other(75)),
		"N5" => Some(Other(76)),
		"N6" => Some(Other(77)),
		"N+" => Some(Other(78)),
		"N1" => Some(Other(79)),
		"N2" => Some(Other(80)),
		"N3" => Some(Other(81)),
		"N0" => Some(Other(82)),
		"N." => Some(Other(83)),
		"N/" => Some(Other(98)),
		_ => {
			if string.starts_with("(") && string.ends_with(")") {
				let parsed = i32::from_str_radix(string.substring(1, string.len() - 1), 10);
				if parsed.is_ok() {
					return Some(Other(parsed.unwrap()));
				}
			}
			Keyboard::from_str(&string).ok()
		}
	}
	
}