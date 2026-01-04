use socket::{send_exit, send_socket};
use util::pulseaudio::{load_null_sink, loopback, set_volume_percentage};
use state::Scanning;
use util::threads::spawn_scan_thread;
use clap::{command, Arg, ArgAction, Command};

use crate::{state::acquire, util::threads::{spawn_drawing_thread, spawn_listening_thread, spawn_pacat_wave_thread, spawn_signal_thread, spawn_socket_thread}};
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
		.arg(Arg::new("no-save").long("no-save").help("disable auto-save of config when the program exits").action(ArgAction::SetTrue))
		.arg(Arg::new("fast-scan").long("fast-scan").help("scan files by extensions instead of header").action(ArgAction::SetTrue))
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
		.subcommand(Command::new("play-wave").about("play a waveform by user-defined ID").arg(Arg::new("id").required(true)))
		.subcommand(Command::new("stop").about("stop all playing files"))
		.subcommand(Command::new("stop-wave").about("stop a waveform by user-defined ID").arg(Arg::new("id").required(true)))
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
		let response = send_socket(subcommand.unwrap())?;
		if response.starts_with("Success") {
			println!("{}", response);
			return Ok(());
		} else {
			panic!("{}", response);
		}
	}
	// Initialize global app object
	let mut app = state::init_app(matches.get_flag("hidden"), matches.get_flag("edit"));

	if app.hidden && app.edit {
		// Mutually exclusive options
		println!("`hidden` is read-only, but `edit` is write-only.");
		println!("You probably don't want this");
		return Ok(());
	}

	// PulseAudio setup
	if !app.edit {
		app.module_null_sink = load_null_sink()?;
		if app.config.loopback_default {
			app.module_loopback_default = loopback("@DEFAULT_SINK@".to_string())?;
		}
		if !app.config.loopback_1.is_empty() {
			app.module_loopback_1 = loopback(app.config.loopback_1.clone())?;
		}
		if !app.config.loopback_2.is_empty() {
			app.module_loopback_2 = loopback(app.config.loopback_2.clone())?;
		}
	}
	set_volume_percentage(app.config.volume);

	let (is_edit, is_hidden) = (app.edit, app.hidden);
	drop(app);

	// Create threads for all background listeners
	spawn_signal_thread()?;
	spawn_scan_thread(Scanning::All);
	let listen_thread = spawn_listening_thread();
	let socket_thread = spawn_socket_thread();
	// Wave playing thread
	if !is_edit {
		spawn_pacat_wave_thread();
	}
	if !is_hidden {
		// If not hidden, we need to render the UI
		let draw_thread = spawn_drawing_thread();
		draw_thread.join().ok();
	}
	// Wait for all threads to end before closing
	listen_thread.join().ok();
	if socket_thread.is_ok() {
		send_exit().ok();
		socket_thread.unwrap().join().ok();
	}

	// Finish up PulseAudio
	{ acquire().unload_modules(); }
	if !is_hidden && !matches.get_flag("no-save") {
		// Save config if not hidden
		config::save();
	}
	Ok(())
}