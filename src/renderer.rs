use ratatui::{
	layout::{Alignment, Constraint, Direction, Layout, Rect}, style::{Color, Style}, widgets::{Block, BorderType, Borders, Paragraph}, Frame
};

use crate::{component::{block::{files::FilesBlock, help::HelpBlock, playing::PlayingBlock, settings::SettingsBlock, tabs::TabsBlock, volume::VolumeBlock, waves::WavesBlock, BlockRender, BlockRenderArea, BlockSingleton}, popup::{popups, PopupRender}}, state::acquire};

pub fn ui(f: &mut Frame) {
	let app = acquire();
	if !app.error.is_empty() {
		return draw_error(f);
	}

 	let chunks = Layout::default()
		.direction(Direction::Vertical)
		.margin(1)
		.constraints(
			[
				Constraint::Length(6),
				Constraint::Length(3),
				Constraint::Fill(1),
				Constraint::Length(1)
			].as_ref()
		)
		.split(f.area());

	let settings = app.settings_opened;
	let waves = app.waves_opened;
	drop(app);
	{ VolumeBlock::instance().render_area(f, chunks[0]); }
	{ TabsBlock::instance().render_area(f, chunks[1]); }
	let files_area: Rect;
	if settings {
		let mid_chunks1 = Layout::default().direction(Direction::Horizontal).constraints([Constraint::Fill(1), Constraint::Length(20)].as_ref()).split(chunks[2]);
		let mid_chunks2 = Layout::default().direction(Direction::Horizontal).constraints([Constraint::Fill(1), Constraint::Percentage(30)].as_ref()).split(chunks[2]);
		let mid_chunks;
		// Settings have minimum 20 char width
		if mid_chunks1[1].width > mid_chunks2[1].width {
			mid_chunks = mid_chunks1;
		} else {
			mid_chunks = mid_chunks2;
		}
		files_area = mid_chunks[0];
		{ SettingsBlock::instance().render_area(f, mid_chunks[1]); }
	} else {
		files_area = chunks[2];
	}
	if waves {
		WavesBlock::instance().render_area(f, files_area);
	} else {
		FilesBlock::instance().render_area(f, files_area);
	}
	{ HelpBlock::instance().render_area(f, chunks[3]); }
	{ PlayingBlock::instance().render(f); }
	popups().iter().for_each(|popup| {
		popup.render(f);
	});
}

fn draw_error(f: &mut Frame) {
	let app = acquire();
	let paragraph = Paragraph::new(app.error.clone())
		.alignment(Alignment::Center)
		.style(Style::default().fg(Color::Red))
		.block(
			Block::default()
				.borders(Borders::ALL)
				.border_type(BorderType::Rounded)
		);
	f.render_widget(paragraph, f.area());
}