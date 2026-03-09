use std::{io, thread::{self, JoinHandle}};

use crossterm::{event::{DisableMouseCapture, EnableMouseCapture}, execute, terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode}};
use ratatui::{
	Frame, Terminal, layout::{Alignment, Constraint, Direction, Layout, Rect}, prelude::CrosstermBackend, style::{Color, Style}, widgets::{Block, BorderType, Borders, Paragraph}
};

use crate::{component::{block::{BlockRender, BlockRenderArea, BlockSingleton, dialogs::DialogBlock, files::FilesBlock, help::HelpBlock, info::InfoBlock, log::{self, LogBlock}, playing::PlayingBlock, settings::SettingsBlock, tabs::TabsBlock, waves::WavesBlock}, popup::{PopupRender, popups}}, constant::{MIN_HEIGHT, MIN_WIDTH}, state::{MainOpened, acquire, is_running, wait_redraw}};

pub fn draw_loop() -> JoinHandle<Result<(), io::Error>> {
	log::info("Spawning drawing thread...");
	return thread::spawn(move || -> Result<(), io::Error> {
		// Setup terminal
		enable_raw_mode()?;
		let mut stdout = io::stdout();
		execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
		let backend = CrosstermBackend::new(stdout);
		let mut terminal = Terminal::new(backend)?;

		// Check minimum terminal size
		let size = terminal.size()?;
		if size.width < MIN_WIDTH || size.height < MIN_HEIGHT {
			let width = size.width;
			let height = size.height;
			let mut app = acquire();
			app.error = String::from(format!("Terminal size requires at least {MIN_WIDTH}x{MIN_HEIGHT}.\nCurrent size: {width}x{height}"));
			app.error_important = true;
		}

		// Render to the terminal
		while is_running() {
			wait_redraw();
			// Render again
			terminal.draw(|f| { ui(f); })?;
		}

		// Restore terminal
		disable_raw_mode()?;
		execute!(
			terminal.backend_mut(),
			LeaveAlternateScreen,
			DisableMouseCapture
		)?;
		terminal.show_cursor()?;
		Ok(())
	});
}

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
				Constraint::Length(7),
				Constraint::Length(3),
				Constraint::Fill(1),
				Constraint::Length(1)
			].as_ref()
		)
		.split(f.area());

	let settings = app.settings_opened;
	let main_opened = app.main_opened;
	drop(app);
	if main_opened == MainOpened::Log {
		LogBlock::instance().render_area(f, f.area());
		return;
	}
	{ InfoBlock::instance().render_area(f, chunks[0]); }
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
	match main_opened {
		MainOpened::File => FilesBlock::instance().render_area(f, files_area),
		MainOpened::Wave => WavesBlock::instance().render_area(f, files_area),
		MainOpened::Dialog => DialogBlock::instance().render_area(f, files_area),
		_ => ()
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