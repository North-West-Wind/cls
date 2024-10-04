use std::{cmp::{max, min}, path::Path};

use ratatui::{
	layout::{Alignment, Constraint, Direction, Layout, Rect}, style::{Color, Modifier, Style, Styled}, text::{Line, Span, Text}, widgets::{Block, BorderType, Borders, Clear, Padding, Paragraph, Widget}, Frame
};
use substring::Substring;

use crate::{constant::{APP_NAME, APP_VERSION}, state::{self, get_app, AwaitInput, InputMode, Popup, Scanning, SelectionLayer}};

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
	if app.input_mode == InputMode::EDITING {
		draw_input(f);
	} else {
		match app.popup {
			Popup::HELP => draw_help_block(f),
			Popup::QUIT => draw_quit_block(f),
			Popup::DELETE_TAB => draw_delete_tab_block(f),
			_ => ()
		}
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

fn border_style(id: u8) -> Style {
	let app = state::get_app();
	Style::default().fg(
		if app.block_selected == id {
			Color::White
		} else {
			Color::DarkGray
		}
	)
}

fn border_type(id: u8) -> BorderType {
	let app = state::get_app();
	if app.block_selected == id && app.selection_layer == SelectionLayer::CONTENT {
		BorderType::Double
	} else {
		BorderType::Rounded
	}
}

fn draw_volume_block(f: &mut Frame, area: Rect) {
	let app = get_app();
	let block = Block::default()
		.title("Volume")
		.borders(Borders::ALL)
		.border_type(border_type(0))
		.border_style(border_style(0))
		.padding(Padding::horizontal(1));
	let mut spans = vec![];
	spans.push(Span::from(format!("Sink Volume ({:0>3}%) ", app.config.volume)));
	let verticals = min(app.config.volume as usize, 100);
	spans.push(Span::from(vec!["|"; verticals].join("")).style(Style::default().fg(if app.config.volume > 100 {
		Color::Red
	} else {
		Color::LightGreen
	})));
	spans.push(Span::from(vec!["-"; 100 - verticals].join("")).style(Style::default().fg(if app.config.volume > 100 {
		Color::Red
	} else {
		Color::Green
	})));
	let paragraph = Paragraph::new(Line::from(spans))
		.block(block);
	f.render_widget(paragraph, area);
}

fn draw_tabs_block(f: &mut Frame, area: Rect) {
	let app = get_app();
	let tabs = app.config.tabs.clone();
	let tab_selected = app.tab_selected as usize;

	let mut total_length: usize = 0;
	let mut spans: Vec<Span> = vec![];
	for (ii, tab) in tabs.iter().enumerate() {
		if total_length as u16 >= area.width - 7 {
			spans.push(Span::from("...").style(Style::default().fg(Color::Green)));
			break;
		}
		let path = Path::new(tab.as_str());
		let basename = path.file_name();
		spans.push(Span::from(basename.unwrap().to_str().unwrap().to_string())
			.style(if ii == tab_selected {
				Style::default().fg(Color::LightGreen).add_modifier(Modifier::REVERSED)
			} else {
				Style::default().fg(Color::Green)
			})
		);
		if ii < tabs.len() - 1 {
			spans.push(Span::from(" | "));
		}
		total_length += basename.unwrap().len() + 3;
	}
	
	let block = Block::default()
		.title("Tabs")
		.borders(Borders::ALL)
		.border_type(border_type(1))
		.border_style(border_style(1));
	let paragraph = Paragraph::new(Line::from(spans)).block(block.padding(Padding::horizontal(1)));
	f.render_widget(paragraph, area);
}

fn draw_files_block(f: &mut Frame, area: Rect) {
	let block = Block::default()
		.title("Files")
		.borders(Borders::ALL)
		.border_type(border_type(2))
		.border_style(border_style(2))
		.padding(Padding::new(2, 2, 1, 1));

	let app = get_app();
	let paragraph: Paragraph;
	if app.scanning == Scanning::ALL {
		paragraph = Paragraph::new("Performing initial scan...");
	} else if app.config.tabs.len() == 0 {
		paragraph = Paragraph::new("Add a tab to get started :>");
	} else if app.scanning == Scanning::ONE(app.tab_selected) {
		paragraph = Paragraph::new("Scanning this directory...\nComeback later :>");
	} else {
		let tab = app.config.tabs[app.tab_selected].clone();
		let files = app.files.as_ref().unwrap().get(&tab);
		if files.is_none() {
			paragraph = Paragraph::new("Failed to read this directory :<\nDoes it exist? Is it readable?");
		} else if files.unwrap().len() == 0 {
			paragraph = Paragraph::new("There are no playable files in this directory :<");
		} else {
			let mut lines = vec![];
			for (ii, (file, duration)) in files.unwrap().iter().enumerate() {
				let mut spans = vec![];
				if duration.len() == 0 {
					spans.push(Span::from(file));
				} else if file.len() + duration.len() > area.width as usize - 6 {
					spans.push(Span::from(file.substring(0, area.width as usize - 10 - duration.len())));
					spans.push(Span::from("... ".to_owned() + duration));
				} else {
					spans.push(Span::from(file.clone()));
					spans.push(Span::from(vec![" "; area.width as usize - 6 - file.len() - duration.len()].join("")));
					spans.push(Span::from(duration.clone()));
				}
				lines.push(Line::from(spans).centered().style(if app.file_selected == ii {
					Style::default().fg(Color::LightBlue).add_modifier(Modifier::REVERSED)
				} else {
					Style::default().fg(Color::Cyan)
				}));
			}
			paragraph = Paragraph::new(lines);
		}
	}
	f.render_widget(paragraph.block(block), area);
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

		Line::from("Root Key Binds").centered(),
		Line::from("? - Help"),
		Line::from("q / esc - Escape / Quit"),
		Line::from("arrow keys - Navigate"),
		Line::from("enter - Select block"),

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
		Line::from("enter - Play file"),
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
	Clear.render(popup_area, f.buffer_mut());
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
	Clear.render(popup_area, f.buffer_mut());
	f.render_widget(Paragraph::new(text).block(Block::bordered().title("Quit?").padding(Padding::horizontal(1)).border_type(BorderType::Rounded).border_style(Style::default().fg(Color::Yellow))), popup_area);
}

fn draw_delete_tab_block(f: &mut Frame) {
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
	Clear.render(popup_area, f.buffer_mut());
	f.render_widget(Paragraph::new(text).block(Block::bordered().title("Delete?").padding(Padding::horizontal(1)).border_type(BorderType::Rounded).border_style(Style::default().fg(Color::Yellow))), popup_area);
}

fn draw_input(f: &mut Frame) {
	let app = get_app();
	if app.input.is_none() {
		return;
	}
	let area = f.area();
	let width = (area.width / 2).max(5) - 5;
	let height = 3;
	let input = app.input.as_ref().unwrap();
	let scroll = input.visual_scroll(width as usize);
	let input_para = Paragraph::new(input.value())
		.scroll((0, scroll as u16))
		.block(Block::bordered().border_type(BorderType::Rounded).title(match app.await_input {
			AwaitInput::ADD_TAB => "Add directory as tab",
			_ => "Input"
		}).padding(Padding::horizontal(1)).style(Style::default().fg(Color::Green)));
	let input_area = Rect {
		x: (area.width - width + 5) / 2,
		y: (area.height - height) / 2,
		width: width + 5,
		height
	};
	Clear.render(input_area, f.buffer_mut());
	f.render_widget(input_para, input_area);
	f.set_cursor_position((
		input_area.x + ((input.visual_cursor()).max(scroll) - scroll) as u16 + 2,
		input_area.y + 1
	));
}