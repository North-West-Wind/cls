use std::sync::{Mutex, MutexGuard, OnceLock};

use ratatui::{layout::Rect, style::{Color, Style}, widgets::Paragraph, Frame};

use crate::{component::block::BlockSingleton, state::acquire, util::global_input::sort_keys};

use super::BlockRenderArea;

pub struct HelpBlock { }

impl BlockSingleton for HelpBlock {
	fn instance() -> MutexGuard<'static, Self> {
		static BLOCK: OnceLock<Mutex<HelpBlock>> = OnceLock::new();
		BLOCK.get_or_init(|| {
			Mutex::new(HelpBlock { })
		}).lock().unwrap()
	}
}

impl BlockRenderArea for HelpBlock {
	fn render_area(&mut self, f: &mut Frame, area: Rect) {
		let app = acquire();
		let mut text = "? for help, q to quit".to_owned();
		if !app.config.stop_key.is_empty() {
			let mut keys = app.config.stop_key.clone().into_iter().collect::<Vec<String>>();
			let keys = sort_keys(&mut keys);
			text += &format!(", {} to stop", keys.join(" + "));
		}
		let paragraph = Paragraph::new(text)
			.style(Style::default().fg(Color::DarkGray));
		f.render_widget(paragraph, area);
	}
}