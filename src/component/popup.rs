use crossterm::event::KeyEvent;
use delete_tab::DeleteTabPopup;
use help::HelpPopup;
use input::InputPopup;
use key_bind::KeyBindPopup;
use mki::Keyboard;
use quit::QuitPopup;
use ratatui::Frame;
use save::SavePopup;

use crate::state::get_mut_app;

pub mod delete_tab;
pub mod help;
pub mod input;
pub mod key_bind;
pub mod quit;
pub mod save;

pub enum PopupComponent {
	DeleteTab(DeleteTabPopup),
	Help(HelpPopup),
	Input(InputPopup),
	KeyBind(KeyBindPopup),
	Quit(QuitPopup),
	Save(SavePopup),
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
		match self {
			PopupComponent::DeleteTab(popup) => popup.render(f),
			PopupComponent::Help(popup) => popup.render(f),
			PopupComponent::Input(popup) => popup.render(f),
			PopupComponent::KeyBind(popup) => popup.render(f),
			PopupComponent::Quit(popup) => popup.render(f),
			PopupComponent::Save(popup) => popup.render(f),
		}
	}
}

impl PopupHandleKey for PopupComponent {
	fn handle_key(&mut self, event: KeyEvent) -> bool {
		match self {
			PopupComponent::DeleteTab(popup) => popup.handle_key(event),
			PopupComponent::Help(popup) => popup.handle_key(event),
			PopupComponent::Input(popup) => popup.handle_key(event),
			PopupComponent::KeyBind(popup) => popup.handle_key(event),
			PopupComponent::Quit(popup) => popup.handle_key(event),
			PopupComponent::Save(popup) => popup.handle_key(event),
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

pub fn exit_popup() {
	let app = get_mut_app();
	app.popup = Option::None;
}

pub fn set_popup(popup: PopupComponent) {
	let app = get_mut_app();
	app.popup = Option::Some(popup);
}