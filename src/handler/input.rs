use crossterm::event::{Event, KeyCode, KeyEvent};
use tui_input::{backend::crossterm::EventHandler, Input};

use crate::state::{get_mut_app, AwaitInput, CondvarPair, InputMode};

use super::block::handle_input_return;

pub fn handle_input_key_event(pair: CondvarPair, event: KeyEvent) -> bool {
	match event.code {
		KeyCode::Enter => complete(pair, true),
		KeyCode::Esc => complete(pair, false),
		_ => {
			get_mut_app().input.as_mut().unwrap().handle_event(&Event::Key(event));
		}
	}
	return true
}

fn complete(pair: CondvarPair, send: bool) {
	let app = get_mut_app();
	let input = get_mut_app().input.as_mut().unwrap();
	if send {
		handle_input_return(pair);
	}
	app.await_input = AwaitInput::NONE;
	app.input_mode = InputMode::NORMAL;
	input.reset();
}

pub fn handle_paste(data: &String) {
	let app = get_mut_app();
	let input = get_mut_app().input.as_mut().unwrap();
	let new_str = input.value().to_owned() + data.as_str();
	app.input = Option::Some(Input::new(new_str));
}