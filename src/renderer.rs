use ratatui::{
	layout::{Alignment, Constraint, Direction, Layout}, style::{Color, Style}, widgets::{Block, BorderType, Borders, Paragraph}, Frame
};

use crate::{component::{block::{BlockRender, BlockRenderArea}, popup::PopupRender}, state};

pub fn ui(f: &mut Frame) {
	let app = state::get_app();
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
	for ii in 0..4 {
		app.blocks[ii].render_area(f, chunks[ii]);
	}
	app.blocks[4].render(f); // playing block render
	if app.popup.is_some() {
		app.popup.as_ref().unwrap().render(f);
	}
}

fn draw_error(f: &mut Frame) {
	let app = state::get_app();
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