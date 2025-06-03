use std::str::FromStr;

use mki::Keyboard;

pub fn keyboard_to_string(keyboard: Keyboard) -> String {
	use Keyboard::*;
	match keyboard {
		Number0 => "0".to_string(),
		Number1 => "1".to_string(),
		Number2 => "2".to_string(),
		Number3 => "3".to_string(),
		Number4 => "4".to_string(),
		Number5 => "5".to_string(),
		Number6 => "6".to_string(),
		Number7 => "7".to_string(),
		Number8 => "8".to_string(),
		Number9 => "9".to_string(),
		Other(code) => match code {
			56 => "LeftAlt".to_string(),
			100 => "RightAlt".to_string(),
			125 => "LeftSuper".to_string(),
			_ => format!("{:?}", keyboard)
		},
		_ => format!("{:?}", keyboard)
	}
}

pub fn string_to_keyboard(string: &str) -> Option<Keyboard> {
	match string {
		"LeftAlt" => Option::Some(Keyboard::Other(56)),
		"RightAlt" => Option::Some(Keyboard::Other(100)),
		"LeftSuper" => Option::Some(Keyboard::Other(125)),
		_ => Keyboard::from_str(&string).ok()
	}
	
}