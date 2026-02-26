use std::{cmp::{max, min}, sync::{LazyLock, Mutex, MutexGuard}, thread};

use crossterm::event::KeyEvent;
use help::HelpPopup;
use input::InputPopup;
use key_bind::KeyBindPopup;
use mki::Keyboard;
use ratatui::{layout::Rect, Frame};
use save::SavePopup;

use crate::component::popup::{confirm::ConfirmPopup, dialog::DialogPopup, wave::WavePopup};

pub mod confirm;
pub mod dialog;
pub mod help;
pub mod input;
pub mod key_bind;
pub mod save;
pub mod wave;

static POPUPS: LazyLock<Mutex<Vec<PopupComponent>>> = LazyLock::new(|| { Mutex::new(vec![]) });

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
	fn handle_key(&mut self, event: KeyEvent) -> bool;
}

pub trait PopupHandlePaste {
	fn handle_paste(&mut self, data: String) -> bool;
}

pub trait PopupHandleGlobalKey {
	fn handle_global_key(&mut self, key: Keyboard);
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
	fn handle_key(&mut self, event: KeyEvent) -> bool {
		use PopupComponent::*;
		match self {
			Confirm(popup) => popup.handle_key(event),
			Help(popup) => popup.handle_key(event),
			Input(popup) => popup.handle_key(event),
			KeyBind(popup) => popup.handle_key(event),
			Save(popup) => popup.handle_key(event),
			Wave(popup) => popup.handle_key(event),
			Dialog(popup) => popup.handle_key(event),
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

impl PopupHandleGlobalKey for PopupComponent {
	fn handle_global_key(&mut self, key: Keyboard) {
		match self {
			PopupComponent::KeyBind(popup) => popup.handle_global_key(key),
			_ => (),
		}
	}
}

impl PopupComponent {
	pub fn has_global_key_handler(&self) -> bool {
		match self {
			PopupComponent::KeyBind(_) => true,
			_ => false,
		}
	}
}

pub fn popups() -> MutexGuard<'static, Vec<PopupComponent>> {
	POPUPS.lock().unwrap()
}

pub fn exit_popup() {
	popups().pop();
}

pub fn set_popup(popup: PopupComponent) {
	popups().push(popup);
}

pub fn defer_exit_popup() {
	thread::spawn(move || {
		exit_popup();
	});
}

pub fn defer_set_popup(popup: PopupComponent) {
	thread::spawn(move || {
		set_popup(popup);
	});
}

pub(self) fn safe_centered_rect(width: u16, height: u16, area: Rect) -> Rect {
	Rect {
		x: max(0, (area.width as i32 - width as i32) / 2) as u16,
		y: max(0, (area.height as i32 - height as i32) / 2) as u16,
		width: min(width, area.width),
		height: min(height, area.height)
	}
}