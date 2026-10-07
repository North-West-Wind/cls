use std::{panic, path::Path, println, thread, time::Duration};

use cpal::traits::{DeviceTrait, HostTrait};
use clap::{command, Arg, ArgAction, Command};
use nng::{Protocol, Socket, options::{Options, RecvTimeout, SendTimeout}};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

use crate::{client::start_client, common::{config, constant::ADDRESS_COMMS, log, socket::{ClientToServer, ServerToClient, decode_s2c, encode_c2s}}, server::start_server};

mod client;
mod common;
mod server;

fn main() -> Result<(), Box<dyn std::error::Error>> {
	// Setup command line to for subcommands and options
	let mut command = command!()
		.about("Command-Line Soundboard")
		.disable_help_flag(true)
		.disable_help_subcommand(true)
		.args_conflicts_with_subcommands(true)
		.arg(Arg::new("help").short('h').long("help").help("print this help menu").action(ArgAction::SetTrue))
		.arg(Arg::new("daemon").short('d').long("daemon").help("run in daemon mode").action(ArgAction::SetTrue))
		.arg(Arg::new("no-save").long("no-save").help("disable auto-save of config when the program exits").action(ArgAction::SetTrue))
		//.arg(Arg::new("fast-scan").long("fast-scan").help("scan files by extensions instead of header").action(ArgAction::SetTrue))
		.arg(Arg::new("no-pacat").long("no-pacat").help("avoid using pacat for playback").action(ArgAction::SetTrue))
		.arg(Arg::new("audio-device").long("audio-device").help("output audio device to use (ignored with pacat)").action(ArgAction::Set))
		.subcommand(Command::new("exit").about("exit another instance"))
		.subcommand(Command::new("audio-devices").about("list available audio devices"))
		.subcommand(Command::new("reload").about("reload config for another instance"))
		.subcommand(Command::new("play").about("play a file").arg(Arg::new("path").required(true)))
		.subcommand(Command::new("play-id").about("play a file by user-defined ID").arg(Arg::new("id").required(true)))
		.subcommand(Command::new("play-wave").about("play a waveform by user-defined ID").arg(Arg::new("id").required(true)))
		.subcommand(Command::new("play-dialog").about("play a dialog by user-defined ID").arg(Arg::new("id").required(true)))
		.subcommand(Command::new("play-search").about("play a searched audio file").arg(Arg::new("query").required(true)))
		.subcommand(Command::new("stop").about("stop all playing files"))
		.subcommand(Command::new("stop-wave").about("stop a waveform by user-defined ID").arg(Arg::new("id").required(true)))
		.subcommand(Command::new("stop-dialog").about("stop a dialog by user-defined ID").arg(Arg::new("id").required(true)));

	// Parse options
	let matches = command.clone().get_matches();
	if matches.get_flag("help") {
		// Specific help option
		command.print_help()?;
		return Ok(());
	}
	// Parse subcommand
	// All subcommands are currently used for IPC
	if let Some((subcommand, matches)) = matches.subcommand() {
		use ClientToServer::*;
		let socket = Socket::new(Protocol::Req0)?;
		socket.set_opt::<SendTimeout>(Some(Duration::from_secs(3)))?;
		socket.set_opt::<RecvTimeout>(Some(Duration::from_secs(3)))?;
		socket.dial(ADDRESS_COMMS)?;
		let result = match subcommand {
			"exit" => {
				let _ = socket.send(&encode_c2s(Exit));
				decode_s2c(&mut socket.recv()?)
			},
			"audio-devices" => {
				list_audio_devices()?;
				return Ok(())
			},
			"reload" => {
				let _ = socket.send(&encode_c2s(Reload));
				decode_s2c(&mut socket.recv()?)
			},
			"play" => {
				let Some(path) = matches.get_one::<String>("path") else { panic!("Missing path") };
				let _ = socket.send(&encode_c2s(PlayPath(path.clone())));
				decode_s2c(&mut socket.recv()?)
			},
			"play-id" => {
				let Some(id) = matches.get_one::<String>("id") else { panic!("Missing id") };
				let Ok(id) = id.parse::<u32>() else { panic!("Could not parse ID") };
				let config = config::load();
				let path = config.files.par_iter().find_map_any(|(tab, files)| {
					files.par_iter().find_map_any(|(name, file)| {
						if let Some(file_id) = file.id && file_id == id {
							Some(Path::new(tab).join(name).to_str().unwrap().to_string())
						} else {
							None
						}
					})
				});
				let Some(path) = path else { panic!("No file with ID {}", id) };
				let _ = socket.send(&encode_c2s(PlayPath(path)));
				decode_s2c(&mut socket.recv()?)
			},
			"play-wave" => {
				let Some(id) = matches.get_one::<String>("id") else { panic!("Missing id") };
				let Ok(id) = id.parse::<u32>() else { panic!("Could not parse ID") };
				let config = config::load();
				let wave = config.waves.par_iter().find_any(|wave| {
					let Some(wave_id) = wave.id else { return false };
					wave_id == id
				});
				let Some(wave) = wave else { panic!("No wave with ID {}", id) };
				let _ = socket.send(&encode_c2s(PlayWave(wave.uid)));
				decode_s2c(&mut socket.recv()?)
			},
			"play-dialog" => {
				let Some(id) = matches.get_one::<String>("id") else { panic!("Missing id") };
				let Ok(id) = id.parse::<u32>() else { panic!("Could not parse ID") };
				let config = config::load();
				let dialog = config.dialogs.par_iter().find_any(|dialog| {
					let Some(dialog_id) = dialog.id else { return false };
					dialog_id == id
				});
				let Some(dialog) = dialog else { panic!("No wave with ID {}", id) };
				let _ = socket.send(&encode_c2s(PlayDialog(dialog.uid)));
				decode_s2c(&mut socket.recv()?)
			},
			"play-search" => {
				let Some(query) = matches.get_one::<String>("query") else { panic!("Missing query") };
				let _ = socket.send(&encode_c2s(PlaySearch(query.clone())));
				decode_s2c(&mut socket.recv()?)
			},
			"stop" => {
				let _ = socket.send(&encode_c2s(StopFiles));
				decode_s2c(&mut socket.recv()?)
			},
			"stop-wave" => {
				let Some(id) = matches.get_one::<String>("id") else { panic!("Missing id") };
				let Ok(id) = id.parse::<u32>() else { panic!("Could not parse ID") };
				let config = config::load();
				let wave = config.waves.par_iter().find_any(|wave| {
					let Some(wave_id) = wave.id else { return false };
					wave_id == id
				});
				let Some(wave) = wave else { panic!("No wave with ID {}", id) };
				let _ = socket.send(&encode_c2s(StopWave(wave.uid)));
				decode_s2c(&mut socket.recv()?)
			},
			"stop-dialog" => {
				let Some(id) = matches.get_one::<String>("id") else { panic!("Missing id") };
				let Ok(id) = id.parse::<u32>() else { panic!("Could not parse ID") };
				let config = config::load();
				let dialog = config.dialogs.par_iter().find_any(|dialog| {
					let Some(dialog_id) = dialog.id else { return false };
					dialog_id == id
				});
				let Some(dialog) = dialog else { panic!("No wave with ID {}", id) };
				let _ = socket.send(&encode_c2s(StopDialog(dialog.uid)));
				decode_s2c(&mut socket.recv()?)
			},
			_ => panic!("Unknown subcommand {}", subcommand)
		};
		return match result {
			Err(err) => Err(err),
			Ok(ServerToClient::Error(err)) => panic!("{}", err),
			_ => {
				println!("Success");
				Ok(())
			}
		};
	}

	let daemon = matches.get_flag("daemon");
	let no_pacat = matches.get_flag("no-pacat");
	let cpal_device = matches.get_one::<String>("audio-device").map_or(String::new(), |device| device.clone());

	// Start client
	let client_thread = if !daemon {
		let save_on_exit = !matches.get_flag("no-save");
		Some(thread::spawn(move || {
			let _ = start_client(save_on_exit);
		}))
	} else { None };

	// Start server
	let server_thread = thread::spawn(move || {
		let _ = start_server(no_pacat, cpal_device, !daemon);
	});

	// Wait for client to exit
	if let Some(client_thread) = client_thread {
		client_thread.join().unwrap();
		if !server_thread.is_finished() {
			log::info("Server is still running! Keep it running for global hot keys");
			server_thread.join().unwrap();
		}
	} else {
		server_thread.join().unwrap();
	}

	Ok(())
}

fn list_audio_devices() -> Result<(), Box<dyn std::error::Error>> {
	for id in cpal::available_hosts() {
		let host = cpal::host_from_id(id)?;
		let devices = host.output_devices()?;
		for device in devices {
			println!("{}", device.id()?);
		}
	}
	Ok(())
}