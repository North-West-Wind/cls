use ratatui::{
	layout::{Alignment, Constraint, Direction, Layout}, style::{Color, Style}, widgets::{Block, BorderType, Borders, Paragraph}, Frame
};

use crate::state;

pub fn ui(f: &mut Frame) {
	if !unsafe { state::ERROR.is_empty() } {
		return draw_error(f);
	}
 let chunks = Layout::default()
			.direction(Direction::Vertical)
			.margin(1)
			.constraints(
					[
							Constraint::Percentage(20),
							Constraint::Percentage(10),
							Constraint::Percentage(80)
					].as_ref()
			)
			.split(f.area());
	let block = Block::default()
			 .title("Volume")
			 .borders(Borders::ALL)
			 .border_type(BorderType::Rounded);
	f.render_widget(block, chunks[0]);
	let block = Block::default()
			 .title("Tabs")
			 .borders(Borders::ALL)
			 .border_type(BorderType::Rounded);
	f.render_widget(block, chunks[1]);
	let block = Block::default()
			 .title("File")
			 .borders(Borders::ALL)
			 .border_type(BorderType::Rounded);
	f.render_widget(block, chunks[2]);
}

fn draw_error(f: &mut Frame) {
	let paragraph = Paragraph::new(unsafe { state::ERROR.clone() })
		.alignment(Alignment::Center)
		.style(Style::default().fg(Color::Red))
		.block(
			Block::default()
				.borders(Borders::ALL)
				.border_type(BorderType::Rounded)
		);
	f.render_widget(paragraph, f.area());
}