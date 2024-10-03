use ratatui::{
	layout::{Alignment, Constraint, Direction, Layout}, style::{Color, Style}, widgets::{Block, BorderType, Borders, Paragraph}, Frame
};

use crate::state::{self, get_block_selected, get_error, get_selection_layer, SelectionLayer};

pub fn ui(f: &mut Frame) {
	if !get_error().is_empty() {
		return draw_error(f);
	}
 let chunks = Layout::default()
			.direction(Direction::Vertical)
			.margin(1)
			.constraints(
					[
						Constraint::Length(5),
						Constraint::Length(3),
						Constraint::Fill(1),
						Constraint::Length(1)
					].as_ref()
			)
			.split(f.area());
	let block = Block::default()
			 .title("Volume")
			 .borders(Borders::ALL)
			 .border_type(BorderType::Rounded)
			 .border_style(border_style(0));
	f.render_widget(block, chunks[0]);
	let block = Block::default()
			 .title("Tabs")
			 .borders(Borders::ALL)
			 .border_type(BorderType::Rounded)
			 .border_style(border_style(1));
	f.render_widget(block, chunks[1]);
	let block = Block::default()
			 .title("Files")
			 .borders(Borders::ALL)
			 .border_type(BorderType::Rounded)
			 .border_style(border_style(2));
	f.render_widget(block, chunks[2]);
	let paragraph = Paragraph::new("? for help, q to quit")
			.style(Style::default().fg(Color::DarkGray));
	f.render_widget(paragraph, chunks[3]);
}

fn draw_error(f: &mut Frame) {
	let paragraph = Paragraph::new(get_error())
		.alignment(Alignment::Center)
		.style(Style::default().fg(Color::Red))
		.block(
			Block::default()
				.borders(Borders::ALL)
				.border_type(BorderType::Rounded)
		);
	f.render_widget(paragraph, f.area());
}

fn border_style(id: u8) -> Style {
	Style::default().fg(
		if get_block_selected() == id {
			if get_selection_layer() == SelectionLayer::BLOCK {
				Color::White
			} else {
				Color::Yellow
			}
		} else {
			Color::DarkGray
		}
	)
}