use crossterm::event::{Event, KeyCode, KeyEvent};
use tui_input::backend::crossterm::EventHandler;

use crate::state::{get_mut_app, AwaitInput, InputMode};

use super::block::handle_input_return;

pub fn handle_input_key_event(event: KeyEvent) -> bool {
	match event.code {
		KeyCode::Enter => complete(true),
		KeyCode::Esc => complete(false),
		_ => {
			get_mut_app().input.as_mut().unwrap().handle_event(&Event::Key(event));
		}
	}
	return true
}

fn complete(send: bool) {
	let app = get_mut_app();
	let input = get_mut_app().input.as_mut().unwrap();
	if send {
		handle_input_return();
	}
	app.await_input = AwaitInput::NONE;
	app.input_mode = InputMode::NORMAL;
	input.reset();
}