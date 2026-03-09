use std::{f32::consts::PI, io::{self, BufWriter, Write}, process::{Child, ChildStdin, Command, Stdio}, sync::{LazyLock, Mutex, MutexGuard}, thread, time::{Duration, SystemTime}};

use crate::{constant::APP_NAME, state::{acquire, is_running, notify_redraw}, util::{file::acquire_playing_files, waveform::{WaveType, acquire_playing_waves}}};

const CHUNK_SIZE: usize = 1024;

struct Pacat {
	last_used: SystemTime,
	child: Child,
	writer: BufWriter<ChildStdin>,
	discarded: bool,
}

fn spawn_pacat(channels: u8) -> Pacat {
	let mut child = Command::new("pacat").args([
		"-d",
		APP_NAME,
		format!("--channels={}", channels).as_str(),
		"--rate=48000",
		"--format=float32le",
		format!("--latency={}", CHUNK_SIZE).as_str()
	])
		.stdin(Stdio::piped())
		.stdout(Stdio::piped()).spawn().expect("Failed to spawn pacat process");
	let stdin = child.stdin.take().unwrap();
	Pacat {
		last_used: SystemTime::now(),
		child: child,
		writer: BufWriter::with_capacity(CHUNK_SIZE * 4, stdin),
		discarded: false,
	}
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PlayerType {
	File,
	Wave
}

fn get_pacat(player_type: PlayerType) -> MutexGuard<'static, Pacat> {
	static FILES: LazyLock<Mutex<Pacat>> = LazyLock::new(|| Mutex::new(spawn_pacat(2)));
	static WAVES: LazyLock<Mutex<Pacat>> = LazyLock::new(|| Mutex::new(spawn_pacat(1)));
	use PlayerType::*;
	match player_type {
		File => FILES.lock().unwrap(),
		Wave => WAVES.lock().unwrap()
	}
}

pub fn create_audio_player(player_type: PlayerType) {
	thread::spawn(move || {
		use PlayerType::*;
		let mut buf = [0_u8; CHUNK_SIZE * 4];
		while is_running() {
			let available = match player_type {
				File => get_file_data(&mut buf),
				Wave => get_wave_data(&mut buf),
			};
			let mut pacat = get_pacat(player_type);
			if available {
				if pacat.discarded {
					// Pacat is killed. Needs respawn
					*pacat = spawn_pacat(2);
				}

				pacat.writer.write_all(&buf).expect("Failed to write to pacat stdin");
				drop(pacat);
				// If blocked, we wait
				loop {
					let mut pacat = get_pacat(player_type);
					if let Err(err) = pacat.writer.flush() {
						if err.kind() != io::ErrorKind::WouldBlock {
							break;
						}
						thread::sleep(Duration::from_millis(10));
					} else {
						pacat.last_used = SystemTime::now();
						break;
					}
				}
			} else if !pacat.discarded {
				if SystemTime::now().duration_since(pacat.last_used).expect("Failed to get pacat duration").as_secs() > 5 {
					pacat.child.kill().expect("Failed to kill pacat");
					pacat.discarded = true;
				}
				drop(pacat);
			}
			thread::sleep(Duration::from_millis(10));
		}
		let mut pacat = get_pacat(player_type);
		if !pacat.discarded {
			pacat.child.kill().expect("Failed to kill pacat");
		}
	});
}

fn get_file_data(buf: &mut [u8; CHUNK_SIZE * 4]) -> bool {
	let mut playing_files = acquire_playing_files();
	let mut eofs = vec![];
	if playing_files.len() > 0 {
		let mut sum_bytes = [0_f32; CHUNK_SIZE];
		for (uuid, playable) in playing_files.iter_mut() {
			let max_read = CHUNK_SIZE.min(playable.data.len() - playable.position);
			for ii in 0..max_read {
				sum_bytes[ii] += playable.data[ii + playable.position] * playable.volume;
			}
			playable.position += max_read;
			if playable.position == playable.data.len() {
				let (lock, cvar) = &*playable.finished;
				let _locked = lock.lock().expect("Failed to lock conditional variable");
				cvar.notify_one();
				eofs.push(*uuid);
				continue;
			}
		}
		if !eofs.is_empty() {
			let mut app = acquire();
			eofs.iter().for_each(|uuid| {
				playing_files.remove(uuid);
				app.playing_file.remove(uuid);
				notify_redraw();
			});
		}
		drop(playing_files);

		for ii in 0..CHUNK_SIZE {
			[
				buf[ii * 4],
				buf[ii * 4 + 1],
				buf[ii * 4 + 2],
				buf[ii * 4 + 3]
			] = sum_bytes[ii].to_le_bytes();
		}
		return true;
	}
	false
}

fn get_wave_data(buf: &mut [u8; CHUNK_SIZE * 4]) -> bool {
	let mut playing_waves = acquire_playing_waves();
	if playing_waves.len() > 0 {
		let mut sum_bytes = [0_f32; CHUNK_SIZE];
		for (_uuid, playable) in playing_waves.iter_mut() {
			let len = playable.len() as f32;
			let mut playable_bytes = [0_f32; CHUNK_SIZE];
			for wave in playable {
				for ii in 0..CHUNK_SIZE {
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
			for ii in 0..CHUNK_SIZE {
				sum_bytes[ii] += playable_bytes[ii] / len;
			}
		}
		
		for ii in 0..CHUNK_SIZE {
			[
				buf[ii * 4],
				buf[ii * 4 + 1],
				buf[ii * 4 + 2],
				buf[ii * 4 + 3]
			] = sum_bytes[ii].to_le_bytes();
		}
		return true;
	}
	false
}