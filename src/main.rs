use std::{io, thread::{self, JoinHandle}, time::Duration};
use constant::{MIN_HEIGHT, MIN_WIDTH};
use signal_hook::iterator::Signals;
use socket::{ensure_socket, listen_socket, send_exit, send_socket, socket_path};
use util::pulseaudio::{load_null_sink, loopback, set_volume_percentage};
use listener::{listen_events, listen_global_input};
use ratatui::{
	backend::CrosstermBackend,
	Terminal
};
use crossterm::{
	event::{DisableMouseCapture, EnableMouseCapture},
	execute,
	terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use state::{config, get_mut_app, Scanning};
use util::threads::spawn_scan_thread;
use clap::{command, Arg, ArgAction, Command};
mod component;
mod config;
mod constant;
mod listener;
mod renderer;
mod socket;
mod state;
mod util;

fn main() -> Result<(), Box<dyn std::error::Error>> {
	// Setup command line to for subcommands and options
	let mut command = command!()
		.about("Command-Line Soundboard")
		.disable_help_flag(true)
		.disable_help_subcommand(true)
		.args_conflicts_with_subcommands(true)
		.arg(Arg::new("help").short('h').long("help").help("print this help menu").action(ArgAction::SetTrue))
		.arg(Arg::new("edit").short('e').long("edit").help("run the soundboard in edit mode, meaning you can only modify config and not play anything").action(ArgAction::SetTrue))
		.arg(Arg::new("hidden").long("hidden").help("run the soundboard in the background, basically read-only").action(ArgAction::SetTrue))
		.subcommand(Command::new("exit").about("exit another instance"))
		.subcommand(Command::new("reload-config").about("reload config for another instance"))
		.subcommand(Command::new("add-tab").about("add a directory tab").arg(Arg::new("dir").required(true)))
		.subcommand(Command::new("delete-tab").about("delete a tab, defaults to the selected one")
			.args([
				Arg::new("index").long("index").help("delete a specific index (starting at 0)"),
				Arg::new("path").long("path").help("delete the tab with this path"),
				Arg::new("name").long("name").help("delete the tab with this basename")
			]))
		.subcommand(Command::new("reload-tab").about("reload a tab, defaults to the selected one").args([
			Arg::new("index").long("index").help("reload a specific index (starting at 0)"),
			Arg::new("path").long("path").help("reload the tab with this path"),
			Arg::new("name").long("name").help("reload the tab with this basename")
		]))
		.subcommand(Command::new("play").about("play a file").arg(Arg::new("path").required(true)))
		.subcommand(Command::new("play-id").about("play a file by user-defined ID").arg(Arg::new("id").required(true)))
		.subcommand(Command::new("stop").about("stop all playing files"))
		.subcommand(Command::new("set-volume").about("set volume of the sink or a file").args([
			Arg::new("volume").help("new volume or volume increment (-200 - +200)"),
			Arg::new("increment").long("increment").help("increment volume instead of setting it").action(ArgAction::SetTrue),
			Arg::new("path").long("path").help("a file's volume to set")
		]));

	// Parse options
	let matches = command.clone().get_matches();
	if matches.get_flag("help") {
		// Specific help option
		command.print_help()?;
		return Ok(());
	}
	// Parse subcommand
	// All subcommands are currently used for IPC
	let subcommand = matches.subcommand();
	if subcommand.is_some() {
		send_socket(subcommand.unwrap())?;
		return Ok(());
	}
	// Initialize global app object
	state::init_app(matches.get_flag("hidden"), matches.get_flag("edit"));
	let app = get_mut_app();

	if app.hidden && app.edit {
		// Mutually exclusive options
		println!("`hidden` is read-only, but `edit` is write-only.");
		println!("You probably don't want this");
		return Ok(());
	}
	if !app.edit {
		// If not write-only, ensure we can use the socket
		ensure_socket();
		if !app.socket_holder {
			println!("Found existing socket! That probably means another instance is running. Forcing edit mode...");
			println!("If there isn't another instance running, delete {}", socket_path().to_str().unwrap());
			app.edit = true;
			thread::sleep(Duration::from_secs(3));
		}
	}

	// PulseAudio setup
	let config = config();
	if !app.edit {
		app.module_null_sink = load_null_sink()?;
		if config.loopback_default {
			app.module_loopback_default = loopback("@DEFAULT_SINK@".to_string())?;
		}
		if !config.loopback_1.is_empty() {
			app.module_loopback_1 = loopback(config.loopback_1.clone())?;
		}
		if !config.loopback_2.is_empty() {
			app.module_loopback_2 = loopback(config.loopback_2.clone())?;
		}
	}
	set_volume_percentage(config.volume);

	app.running = true;

	// Create threads for all background listeners
	spawn_signal_thread()?;
	spawn_scan_thread(Scanning::All);
	let listen_thread = spawn_listening_thread();
	let mut socket_thread = Option::None;
	if app.socket_holder {
		socket_thread = Option::Some(spawn_socket_thread());
	}
	if !app.hidden {
		// If not hidden, we need to render the UI
		let draw_thread = spawn_drawing_thread();
		draw_thread.join().ok();
	}
	// Wait for all threads to end before closing
	listen_thread.join().ok();
	if socket_thread.is_some() {
		send_exit().ok();
		socket_thread.unwrap().join().ok();
	}

	// Finish up PulseAudio
	app.unload_modules();
	if !app.hidden {
		// Save config if not hidden
		config::save();
	}
	Ok(())
}

fn spawn_drawing_thread() -> JoinHandle<Result<(), io::Error>> {
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
			let app = state::get_mut_app();
			app.error = String::from(format!("Terminal size requires at least {MIN_WIDTH}x{MIN_HEIGHT}.\nCurrent size: {width}x{height}"));
			app.error_important = true;
		}

		// Render to the terminal
		let app = state::get_app();
		let pair = app.pair.clone();
		while app.running {
			let (lock, cvar) = &*pair;
			let mut shared = lock.lock().expect("Failed to get shared mutex");
			// Wait for redraw notice
			while !(*shared).redraw {
				shared = cvar.wait(shared).expect("Failed to get shared mutex");
			}
			(*shared).redraw = false;
			// Render again
			terminal.draw(|f| { renderer::ui(f); })?;
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

// A thread for listening for inputs
fn spawn_listening_thread() -> JoinHandle<()> {
	return thread::spawn(move || {
		listen_global_input();
		listen_events().ok();
	});
}

// A thread for listening for signals
fn spawn_signal_thread() -> Result<JoinHandle<()>, io::Error> {
	use signal_hook::consts::*;
	let mut signals = Signals::new([SIGINT, SIGTERM])?;
	return Ok(thread::spawn(move || {
		for sig in signals.forever() {
			let app = get_mut_app();
			match sig {
				SIGINT|SIGTERM => {
					app.running = false;
					break;
				},
				_ => (),
			}
		}
	}));
}

// A thread for listening for socket (IPC)
fn spawn_socket_thread() -> JoinHandle<()> {
	return thread::spawn(move || {
		listen_socket().ok();
	});
}