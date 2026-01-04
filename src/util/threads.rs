use std::{f32::consts::PI, io::{self, Write}, process::{Child, Command, Stdio}, sync::{LazyLock, Mutex, MutexGuard}, thread::{self, JoinHandle}, time::{Duration, SystemTime}};

use crossterm::{event::{DisableMouseCapture, EnableMouseCapture}, execute, terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen}};
use ratatui::{prelude::CrosstermBackend, Terminal};
use signal_hook::iterator::Signals;

use crate::{component::popup::{exit_popup, save::SavePopup, set_popup, PopupComponent}, config, constant::{APP_NAME, MIN_HEIGHT, MIN_WIDTH}, listener::{listen_events, listen_global, unlisten_global}, renderer, socket::{listen_socket, try_socket}, state::{acquire, acquire_running, notify_redraw, notify_respawn, wait_redraw, wait_respawn, Scanning}, util::{self, waveform::{acquire_playing_waves, WaveType}}};

pub fn spawn_drawing_thread() -> JoinHandle<Result<(), io::Error>> {
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
			if !acquire().attached {
				break;
			}
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

// A thread that respawns the drawing thread
pub fn spawn_respawn_thread() -> JoinHandle<()> {
	return thread::spawn(move || {
		while *acquire_running() {
			let hidden = { acquire().hidden };
			if !hidden {
				spawn_drawing_thread().join().ok();
			}
			wait_respawn();
			println!("Respawn triggered");
		}

		let pid = { acquire().forked };
		if pid != 0 {
			println!("Detached child {pid}");
			std::process::exit(0);
		}
	});
}

// A thread for listening for inputs
pub fn spawn_listening_thread() -> JoinHandle<()> {
	return thread::spawn(move || {
		listen_global();
		listen_events().ok();
		unlisten_global();
	});
}

// A thread for listening for signals
pub fn spawn_signal_thread() -> Result<JoinHandle<()>, io::Error> {
	use signal_hook::consts::*;
	let mut signals = Signals::new([SIGINT, SIGTERM])?;
	return Ok(thread::spawn(move || {
		for sig in signals.forever() {
			match sig {
				SIGINT|SIGTERM => {
					*acquire_running() = false;
					notify_respawn();
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
		let _ = match mode {
				Scanning::All => util::scan_tabs(),
				Scanning::One(index) => util::scan_tab(index),
				_ => Ok(())
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

struct WavePacat {
	last_used: SystemTime,
	child: Child
}

static PACAT: LazyLock<Mutex<WavePacat>> = LazyLock::new(|| {
	let pacat = WavePacat {
		last_used: SystemTime::now(),
		child: Command::new("pacat").args([
			"-d",
			APP_NAME,
			"--channels=1",
			"--rate=48000",
			"--format=float32le",
			format!("--latency={}", WAVE_CHUNK).as_str()
		])
			.stdin(Stdio::piped())
			.stdout(Stdio::piped()).spawn().expect("Failed to spawn pacat process")
	};
	Mutex::new(pacat)
});

fn acquire_pacat() -> MutexGuard<'static, WavePacat> {
	let pacat = PACAT.lock().unwrap();
	pacat
}

const WAVE_CHUNK: usize = 1600;

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

				// Normalize wave when too loud
				let max = sum_bytes.iter().fold(0.0, |acc, byte| { byte.abs().max(acc) });
				if max > 2.0 {
					for ii in 0..WAVE_CHUNK {
						sum_bytes[ii] /= max / 2.0;
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

				let mut pacat = acquire_pacat();
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
				}

				pacat_running = true;
				pacat.last_used = SystemTime::now();
				let stdin = pacat.child.stdin.as_mut().expect("Failed to get pacat stdin");
				stdin.write_all(&bytes).expect("Failed to write to pacat stdin");
				stdin.flush().expect("Failed to flush pacat stdin");
			} else if pacat_running {
				let mut pacat = acquire_pacat();
				if SystemTime::now().duration_since(pacat.last_used).expect("Failed to get pacat duration").as_secs() > 5 {
					pacat.child.kill().expect("Failed to kill pacat");
					pacat_running = false;
				}
			}
			drop(playing_waves);
			thread::sleep(Duration::from_secs_f32(WAVE_CHUNK as f32 / 48000.0));
		}
		if pacat_running {
			let mut pacat = acquire_pacat();
			pacat.child.kill().expect("Failed to kill pacat");
		}
	});
}
