use socket::{send_exit, send_socket};
use util::pulseaudio::{load_null_sink, loopback};
use state::Scanning;
use clap::{command, Arg, ArgAction, Command};

use crate::{listener::{listen_signals, program_loop}, renderer::draw_loop, socket::start_socket, state::acquire, util::{audio::{PlayerType, create_audio_player, list_audio_devices}, file::audio_cache_invalidator, tab::scan}};
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
		.arg(Arg::new("no-pacat").long("no-pacat").help("avoid using pacat for playback").action(ArgAction::SetTrue))
		.arg(Arg::new("audio-device").long("audio-device").help("output audio device to use (ignored with pacat)").action(ArgAction::Set))
		.subcommand(Command::new("exit").about("exit another instance"))
		.subcommand(Command::new("audio-devices").about("list available audio devices"))
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
		.subcommand(Command::new("play-dialog").about("play a dialog by user-defined ID").arg(Arg::new("id").required(true)))
		.subcommand(Command::new("stop").about("stop all playing files"))
		.subcommand(Command::new("stop-wave").about("stop a waveform by user-defined ID").arg(Arg::new("id").required(true)))
		.subcommand(Command::new("stop-dialog").about("stop a dialog by user-defined ID").arg(Arg::new("id").required(true)))
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
		let (subcommand, matches) = subcommand.unwrap();
		match subcommand {
			"audio-devices" => {
				list_audio_devices()?;
				return Ok(())
			},
			_ => {
				let response = send_socket((subcommand, matches))?;
				if response.starts_with("Success") {
					println!("{}", response);
					return Ok(());
				} else {
					panic!("{}", response);
				}
			}
		}
	}
	// Initialize global app object
	let mut app = acquire();
	(app.hidden, app.edit, app.no_pacat) = (matches.get_flag("hidden"), matches.get_flag("edit"), matches.get_flag("no-pacat"));
	app.cpal_device = matches.get_one::<String>("audio-device").unwrap_or(&String::new()).clone();

	if app.hidden && app.edit {
		// Mutually exclusive options
		println!("`hidden` is read-only, but `edit` is write-only.");
		println!("You probably don't want this");
		return Ok(());
	}

	// PulseAudio setup
	if !app.edit {
		app.module_null_sink = load_null_sink();
		if app.config.loopback_default {
			app.module_loopback_default = loopback("@DEFAULT_SINK@".to_string());
		}
		if !app.config.loopback_1.is_empty() {
			app.module_loopback_1 = loopback(app.config.loopback_1.clone());
		}
		if !app.config.loopback_2.is_empty() {
			app.module_loopback_2 = loopback(app.config.loopback_2.clone());
		}
	}

	let (is_edit, is_hidden) = (app.edit, app.hidden);
	drop(app);

	// Create threads for all background listeners
	listen_signals();
	scan(Scanning::All);
	let socket_thread = start_socket();
	// Audio players
	if !is_edit {
		create_audio_player(PlayerType::File);
		create_audio_player(PlayerType::Wave);
		audio_cache_invalidator();
	}
	let draw_thread = if !is_hidden {
		// If not hidden, we need to render the UI
		Some(draw_loop())
	} else {
		None
	};
	// Keep the program running
	program_loop().ok();
	draw_thread.map(|thread| thread.join());
	// Wait for all threads to end before closing
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