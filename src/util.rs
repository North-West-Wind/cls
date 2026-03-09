use std::{collections::HashMap, path::Path, thread::{self, JoinHandle}};

use ffprobe::FfProbe;
use file_format::{FileFormat, Kind};
use mime_guess::mime;

use crate::{component::block::{files::FilesBlock, tabs::TabsBlock, BlockSingleton}, state::{acquire, notify_redraw}};

pub mod dialog;
pub mod file;
pub mod fs;
pub mod global_input;
pub mod pulseaudio;
pub mod threads;
pub mod waveform;

pub fn selected_file_path(tabs: &Vec<String>, files: &HashMap<String, Vec<(String, String)>>, selected: Option<usize>) -> String {
	let tab_selected = { TabsBlock::instance().selected };
	if tab_selected >= tabs.len() {
		return String::new();
	}
	let tab = tabs[tab_selected].clone();
	let files = files.get(&tab);
	if files.is_none() {
		return String::new();
	}
	let files = files.unwrap();
	let selected = selected.unwrap_or_else(|| { FilesBlock::instance().selected });
	if selected >= files.len() {
		return String::new();
	}
	return Path::new(&tab).join(&files[selected].0).into_os_string().into_string().unwrap();
}