use std::{f32::consts::PI, io::{self, BufWriter, Read, Write}, process::{Child, ChildStdin, Command, Stdio}, sync::{LazyLock, Mutex, MutexGuard}, thread::{self, JoinHandle}, time::{Duration, SystemTime}};

use crossterm::{event::{DisableMouseCapture, EnableMouseCapture}, execute, terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen}};
use ratatui::{prelude::CrosstermBackend, Terminal};
use signal_hook::iterator::Signals;

use crate::{component::{block::log, popup::{PopupComponent, exit_popup, save::SavePopup, set_popup}}, config, constant::{APP_NAME, MIN_HEIGHT, MIN_WIDTH}, listener::{listen_events, listen_global, unlisten_global}, renderer, socket::{listen_socket, try_socket}, state::{Scanning, acquire, acquire_running, notify_redraw, wait_redraw}, util::{self, file::acquire_playing_files, waveform::{WaveType, acquire_playing_waves}}};

pub fn spawn_drawing_thread() -> JoinHandle<Result<(), io::Error>> {
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
		while *acquire_running() {
			wait_redraw();
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
pub fn spawn_listening_thread() -> JoinHandle<()> {
	log::info("Spawning listening thread...");
	return thread::spawn(move || {
		listen_global();
		listen_events().ok();
		unlisten_global();
	});
}

// A thread for listening for signals
pub fn spawn_signal_thread() -> Result<JoinHandle<()>, io::Error> {
	log::info("Spawning signal thread...");
	use signal_hook::consts::*;
	let mut signals = Signals::new([SIGINT, SIGTERM])?;
	return Ok(thread::spawn(move || {
		for sig in signals.forever() {
			match sig {
				SIGINT|SIGTERM => {
					*acquire_running() = false;
					break;
				},
				_ => (),
			}
		}
	}));
}

// A thread for listening for socket (IPC)
pub fn spawn_socket_thread() -> Result<JoinHandle<()>, io::Error> {
	let listener = try_socket()?;
	log::info("Spawning socket thread...");

	Ok(thread::spawn(move || {
		{ acquire().socket_holder = true; }
		listen_socket(listener);
	}))
}

pub fn spawn_scan_thread(mode: Scanning) {
	if mode == Scanning::None {
		return;
	}
	thread::spawn(move || {
		{ acquire().scanning = mode; }
		match mode {
			Scanning::All => {
				log::info("Scanning all tabs...");
				let _ = util::scan_tabs();
				log::info("Scanned all tabs");
			},
			Scanning::One(index) => {
				log::info(format!("Scanning tab {}...", index).as_str());
				util::scan_tab(index);
				log::info(format!("Scanned tab {}", index).as_str());
			},
			_ => ()
		};

		let mut app = acquire();
		app.scanning = Scanning::None;
		notify_redraw();
	});
}

pub fn spawn_save_thread() {
	thread::spawn(move || {
		set_popup(PopupComponent::Save(SavePopup::new(false)));
		notify_redraw();
		config::save();
		exit_popup();
		set_popup(PopupComponent::Save(SavePopup::new(true)));
		notify_redraw();
	});
}

const FILE_CHUNK: usize = 1024;
const WAVE_CHUNK: usize = 512;

struct Pacat {
	last_used: SystemTime,
	child: Child,
	writer: BufWriter<ChildStdin>
}

static FILES: LazyLock<Mutex<Pacat>> = LazyLock::new(|| {
	let mut child = Command::new("pacat").args([
		"-d",
		APP_NAME,
		"--channels=2",
		"--rate=48000",
		"--format=float32le",
		format!("--latency={}", FILE_CHUNK).as_str()
	])
		.stdin(Stdio::piped())
		.stdout(Stdio::piped()).spawn().expect("Failed to spawn pacat process");
	let stdin = child.stdin.take().unwrap();
	let pacat = Pacat {
		last_used: SystemTime::now(),
		child: child,
		writer: BufWriter::with_capacity(1024, stdin)
	};
	Mutex::new(pacat)
});

static WAVES: LazyLock<Mutex<Pacat>> = LazyLock::new(|| {
	let mut child = Command::new("pacat").args([
		"-d",
		APP_NAME,
		"--channels=1",
		"--rate=48000",
		"--format=float32le",
		format!("--latency={}", WAVE_CHUNK).as_str()
	])
		.stdin(Stdio::piped())
		.stdout(Stdio::piped()).spawn().expect("Failed to spawn pacat process");
	let stdin = child.stdin.take().unwrap();
	let pacat = Pacat {
		last_used: SystemTime::now(),
		child: child,
		writer: BufWriter::with_capacity(1024, stdin)
	};
	Mutex::new(pacat)
});

fn acquire_files() -> MutexGuard<'static, Pacat> {
	let pacat = FILES.lock().unwrap();
	pacat
}

fn acquire_waves() -> MutexGuard<'static, Pacat> {
	let pacat = WAVES.lock().unwrap();
	pacat
}

pub fn spawn_pacat_file_thread() {
	thread::spawn(move || {
		let mut pacat_running = false;
		let mut pacat_init = false;

		while *acquire_running() {
			let mut bytes = [0_u8; FILE_CHUNK * 4];
			let mut playing_files = acquire_playing_files();
			let mut eofs = vec![];
			if playing_files.len() > 0 {
				let mut sum_bytes = [0_f32; FILE_CHUNK];
				for (uuid, playable) in playing_files.iter_mut() {
					let Ok(read) = playable.reader.read(&mut bytes) else {
						eofs.push(*uuid);
						continue;
					};
					if read == 0 {
						eofs.push(*uuid);
						continue;
					}
					for (ii, byte) in bytes[0..read].chunks_exact(4).map(|chunk| {
						f32::from_le_bytes(chunk.try_into().unwrap())
					}).enumerate() {
						sum_bytes[ii] += byte * playable.volume;
					}
				}
				eofs.iter().for_each(|uuid| {
					playing_files.remove(uuid);
					acquire().playing_file.remove(uuid);
					notify_redraw();
				});

				for ii in 0..FILE_CHUNK {
					[
						bytes[ii * 4],
						bytes[ii * 4 + 1],
						bytes[ii * 4 + 2],
						bytes[ii * 4 + 3]
					] = sum_bytes[ii].to_le_bytes();
				}

				let mut pacat = acquire_files();
				if !pacat_init {
					// First acquisition of the lock spawns a pacat
					pacat_init = true;
				} else if !pacat_running {
					// Pacat is killed. Needs respawn
					pacat.child = Command::new("pacat").args([
						"-d",
						APP_NAME,
						"--channels=2",
						"--rate=48000",
						"--format=float32le",
						format!("--latency={}", FILE_CHUNK).as_str()
					])
						.stdin(Stdio::piped())
						.stdout(Stdio::piped()).spawn().expect("Failed to respawn pacat process");
					pacat.writer = BufWriter::with_capacity(1024, pacat.child.stdin.take().unwrap());
				}

				pacat_running = true;
				pacat.writer.write_all(&bytes).expect("Failed to write to pacat stdin");
				// If blocked, we wait
				while let Err(err) = pacat.writer.flush() {
					if err.kind() != io::ErrorKind::WouldBlock {
						break;
					}
					thread::sleep(Duration::from_millis(10));
				}
				pacat.last_used = SystemTime::now();
			} else if pacat_running {
				let mut pacat = acquire_files();
				if SystemTime::now().duration_since(pacat.last_used).expect("Failed to get pacat duration").as_secs() > 5 {
					pacat.child.kill().expect("Failed to kill pacat");
					pacat_running = false;
				}
			}
			drop(playing_files);
			thread::sleep(Duration::from_millis(10));
		}
		if pacat_running {
			let mut pacat = acquire_files();
			pacat.child.kill().expect("Failed to kill pacat");
		}
	});
}

pub fn spawn_pacat_wave_thread() {
	thread::spawn(move || {
		let mut pacat_running = false;
		let mut pacat_init = false;

		while *acquire_running() {
			let mut bytes = [0_u8; WAVE_CHUNK * 4];
			let mut playing_waves = acquire_playing_waves();
			if playing_waves.len() > 0 {
				let mut sum_bytes = [0_f32; WAVE_CHUNK];
				for (_uuid, playable) in playing_waves.iter_mut() {
					let len = playable.len() as f32;
					let mut playable_bytes = [0_f32; WAVE_CHUNK];
					for wave in playable {
						for ii in 0..WAVE_CHUNK {
							playable_bytes[ii] += match wave.wave_type {
								WaveType::Sine => (PI * 2.0 * wave.samples as f32 / wave.period as f32).sin(),
								WaveType::Square => if wave.samples as f32 / wave.period as f32 > 0.5 { 1.0 } else { -1.0 },
								WaveType::Triangle => {
									let portion = wave.samples as f32 / wave.period as f32;
									if portion > 0.5 {
										-1.0 + (portion - 0.5) * 4.0
									} else {
										1.0 - portion * 4.0
									}
								},
								WaveType::Saw => -1.0 + (wave.samples as f32 / wave.period as f32) * 2.0,
							} * wave.amplitude;
							wave.samples = (wave.samples + 1) % wave.period;
						}
					}
					for ii in 0..WAVE_CHUNK {
						sum_bytes[ii] += playable_bytes[ii] / len;
					}
				}
				
				for ii in 0..WAVE_CHUNK {
					[
						bytes[ii * 4],
						bytes[ii * 4 + 1],
						bytes[ii * 4 + 2],
						bytes[ii * 4 + 3]
					] = sum_bytes[ii].to_le_bytes();
				}

				let mut pacat = acquire_waves();
				if !pacat_init {
					// First acquisition of the lock spawns a pacat
					pacat_init = true;
				} else if !pacat_running {
					// Pacat is killed. Needs respawn
					pacat.child = Command::new("pacat").args([
						"-d",
						APP_NAME,
						"--channels=1",
						"--rate=48000",
						"--format=float32le",
						format!("--latency={}", WAVE_CHUNK).as_str()
					])
						.stdin(Stdio::piped())
						.stdout(Stdio::piped()).spawn().expect("Failed to respawn pacat process");
					pacat.writer = BufWriter::with_capacity(1024, pacat.child.stdin.take().unwrap());
				}

				pacat_running = true;
				pacat.writer.write_all(&bytes).expect("Failed to write to pacat stdin");
				// If blocked, we wait
				while let Err(err) = pacat.writer.flush() {
					if err.kind() != io::ErrorKind::WouldBlock {
						break;
					}
					thread::sleep(Duration::from_millis(10));
				}
				pacat.last_used = SystemTime::now();
			} else if pacat_running {
				log::info("pacat is running");
				let mut pacat = acquire_waves();
				if SystemTime::now().duration_since(pacat.last_used).expect("Failed to get pacat duration").as_secs() > 5 {
					pacat.child.kill().expect("Failed to kill pacat");
					pacat_running = false;
				}
			}
			drop(playing_waves);
			thread::sleep(Duration::from_millis(10));
		}
		if pacat_running {
			let mut pacat = acquire_waves();
			pacat.child.kill().expect("Failed to kill pacat");
		}
	});
}
