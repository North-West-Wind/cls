use std::cmp::max;

use ratatui::{
	layout::{Alignment, Constraint, Direction, Layout, Rect}, style::{Color, Style}, text::{Line, Text}, widgets::{Block, BorderType, Borders, Padding, Paragraph}, Frame
};

use crate::{constant::{APP_NAME, APP_VERSION}, state::{self, get_block_selected, get_error, get_popup, get_selection_layer, Popup, SelectionLayer}};

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
	draw_volume_block(f, chunks[0]);
	draw_tabs_block(f, chunks[1]);
	draw_files_block(f, chunks[2]);
	draw_help_message(f, chunks[3]);
	match get_popup() {
		Popup::HELP => draw_help_block(f),
		Popup::QUIT => draw_quit_block(f),
		_ => ()
	}
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
			Color::White
		} else {
			Color::DarkGray
		}
	)
}

fn border_type(id: u8) -> BorderType {
	if get_block_selected() == id && get_selection_layer() == SelectionLayer::CONTENT {
		BorderType::Double
	} else {
		BorderType::Rounded
	}
}

fn draw_volume_block(f: &mut Frame, area: Rect) {
	let block = Block::default()
		.title("Volume")
		.borders(Borders::ALL)
		.border_type(border_type(0))
		.border_style(border_style(0));
	f.render_widget(block, area);
}

fn draw_tabs_block(f: &mut Frame, area: Rect) {
	let block = Block::default()
		.title("Tabs")
		.borders(Borders::ALL)
		.border_type(border_type(1))
		.border_style(border_style(1));
	f.render_widget(block, area);
}

fn draw_files_block(f: &mut Frame, area: Rect) {
	let block = Block::default()
		.title("Files")
		.borders(Borders::ALL)
		.border_type(border_type(2))
		.border_style(border_style(2));
	f.render_widget(block, area);
}

fn draw_help_message(f: &mut Frame, area: Rect) {
	let paragraph = Paragraph::new("? for help, q to quit")
		.style(Style::default().fg(Color::DarkGray));
	f.render_widget(paragraph, area);
}

fn draw_help_block(f: &mut Frame) {
	let appname = APP_NAME;
	let text = Text::from(vec![
		Line::from(format!("{appname} - Command Line Soundboard")).centered(),
		Line::from(APP_VERSION).centered(),
		Line::from(""),

		Line::from("Global Key Binds").centered(),
		Line::from("? - Help"),
		Line::from("q / esc - Escape / Quit"),
		Line::from("arrow keys - Navigate"),

		Line::from("Volume Key Binds").centered(),
		Line::from("left - Decrease volume by 1%"),
		Line::from("right - Increase volume by 1%"),
		Line::from("ctrl + left - Decrease volume by 5%"),
		Line::from("ctrl + right - Increase volume by 5%"),

		Line::from("Tabs Key Binds").centered(),
		Line::from("a - Add directory"),
		Line::from("d - Remove directory"),

		Line::from("Files Key Binds").centered(),
		Line::from("r - Refresh"),
		Line::from("return - Play file"),
	]);
	let area = f.area();
	let width = max((text.width() as u16) + 4, area.width / 3);
	let height = max((text.height() as u16) + 2, area.height / 3);
	let popup_area: Rect = Rect {
		x: (area.width - width) / 2,
		y: (area.height - height) / 2,
		width,
		height
	};
	f.render_widget(Paragraph::new(text).block(Block::bordered().padding(Padding::uniform(1)).border_type(BorderType::Rounded)), popup_area);
}

fn draw_quit_block(f: &mut Frame) {
	let text = Text::from(vec![
		Line::from("Press y to quit"),
		Line::from("Press any to cancel")
	]).style(Style::default().fg(Color::Yellow));
	let width = (text.width() as u16) + 4;
	let height = (text.height() as u16) + 2;
	let area = f.area();
	let popup_area: Rect = Rect {
		x: (area.width - width) / 2,
		y: (area.height - height) / 2,
		width,
		height
	};
	f.render_widget(Paragraph::new(text).block(Block::bordered().title("Quit?").padding(Padding::horizontal(1)).border_type(BorderType::Rounded).border_style(Style::default().fg(Color::Yellow))), popup_area);
}