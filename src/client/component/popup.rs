use std::cmp::{max, min};

use crossterm::event::KeyEvent;
use help::HelpPopup;
use input::InputPopup;
use key_bind::KeyBindPopup;
use ratatui::{layout::Rect, Frame};
use save::SavePopup;

use crate::client::{ClientState, component::popup::{confirm::ConfirmPopup, dialog::DialogPopup, wave::WavePopup}};

pub mod confirm;
pub mod dialog;
pub mod help;
pub mod input;
pub mod key_bind;
pub mod save;
pub mod wave;

pub enum PopupComponent {
	Confirm(ConfirmPopup),
	Help(HelpPopup),
	Input(InputPopup),
	KeyBind(KeyBindPopup),
	Save(SavePopup),
	Wave(WavePopup),
	Dialog(DialogPopup),
}

pub trait PopupRender {
	fn render(&self, f: &mut Frame);
}

pub trait PopupHandleKey {
	fn handle_key(&mut self, client_state: &ClientState, event: KeyEvent) -> bool;
}

pub trait PopupHandlePaste {
	fn handle_paste(&mut self, data: String) -> bool;
}

impl PopupRender for PopupComponent {
	fn render(&self, f: &mut Frame) {
		use PopupComponent::*;
		match self {
			Confirm(popup) => popup.render(f),
			Help(popup) => popup.render(f),
			Input(popup) => popup.render(f),
			KeyBind(popup) => popup.render(f),
			Save(popup) => popup.render(f),
			Wave(popup) => popup.render(f),
			Dialog(popup) => popup.render(f),
		}
	}
}

impl PopupHandleKey for PopupComponent {
	fn handle_key(&mut self, client_state: &ClientState, event: KeyEvent) -> bool {
		use PopupComponent::*;
		match self {
			Confirm(popup) => popup.handle_key(client_state, event),
			Help(popup) => popup.handle_key(client_state, event),
			Input(popup) => popup.handle_key(client_state, event),
			KeyBind(popup) => popup.handle_key(client_state, event),
			Save(popup) => popup.handle_key(client_state, event),
			Wave(popup) => popup.handle_key(client_state, event),
			Dialog(popup) => popup.handle_key(client_state, event),
		}
	}
}

impl PopupHandlePaste for PopupComponent {
	fn handle_paste(&mut self, data: String) -> bool {
		match self {
			PopupComponent::Input(popup) => popup.handle_paste(data),
			_ => false,
		}
	}
}

pub(self) fn safe_centered_rect(width: u16, height: u16, area: Rect) -> Rect {
	Rect {
		x: max(0, (area.width as i32 - width as i32) / 2) as u16,
		y: max(0, (area.height as i32 - height as i32) / 2) as u16,
		width: min(width, area.width),
		height: min(height, area.height)
	}
}