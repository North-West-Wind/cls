use std::{f32::consts::PI, io::{self, BufWriter, Write}, process::{Child, ChildStdin, Command, Stdio}, sync::{LazyLock, Mutex, MutexGuard}, thread, time::{Duration, SystemTime}};

use cmd_exists::cmd_exists;
use cpal::{SampleFormat, traits::{DeviceTrait, HostTrait, StreamTrait}};
use lazy_static::lazy_static;

use crate::{constant::{APP_NAME, ENDIANESS}, state::{acquire, is_running, notify_redraw}, util::{file::acquire_playing_files, wave::{WaveType, acquire_playing_waves}}};

const CHUNK_SIZE: usize = 1024;

struct Pacat {
	last_used: SystemTime,
	child: Child,
	writer: BufWriter<ChildStdin>,
	discarded: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PlayerType {
	File,
	Wave
}

lazy_static! {
	static ref USE_PACAT: bool = cmd_exists("pacat").is_ok();
}

fn spawn_pacat(player_type: PlayerType) -> Pacat {
	use PlayerType::*;
	let channels: u8 = match player_type {
		File => 2,
		Wave => 2,
	};
	let mut child = Command::new("pacat").args([
		"-d",
		APP_NAME,
		format!("--channels={}", channels).as_str(),
		"--rate=48000",
		format!("--format=float32{}", ENDIANESS).as_str(),
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

fn get_pacat(player_type: PlayerType) -> MutexGuard<'static, Pacat> {
	static FILES: LazyLock<Mutex<Pacat>> = LazyLock::new(|| Mutex::new(spawn_pacat(PlayerType::File)));
	static WAVES: LazyLock<Mutex<Pacat>> = LazyLock::new(|| Mutex::new(spawn_pacat(PlayerType::Wave)));
	use PlayerType::*;
	match player_type {
		File => FILES.lock().unwrap(),
		Wave => WAVES.lock().unwrap()
	}
}

pub fn create_audio_player(player_type: PlayerType) {
	thread::spawn(move || {
		use PlayerType::*;
		let mut buf = [0_f32; CHUNK_SIZE];
		while is_running() {
			if *USE_PACAT {
				let available = match player_type {
					File => get_file_data(&mut buf),
					Wave => get_wave_data(&mut buf),
				};
				let mut pacat = get_pacat(player_type);
				if available {
					if pacat.discarded {
						// Pacat is killed. Needs respawn
						*pacat = spawn_pacat(player_type);
					}

					pacat.writer.write_all(&bytemuck::cast_slice(&buf)).expect("Failed to write to pacat stdin");
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
					buf.fill(0.0);
				} else if !pacat.discarded {
					if SystemTime::now().duration_since(pacat.last_used).expect("Failed to get pacat duration").as_secs() > 5 {
						pacat.child.kill().expect("Failed to kill pacat");
						pacat.discarded = true;
					}
					drop(pacat);
				}
				thread::sleep(Duration::from_millis(10));
			} else {
				let host = cpal::default_host();
				let device = host.default_output_device().expect("Failed to get default output device");
				let config = device.supported_output_configs().unwrap()
					.find(|config| {
						config.sample_format() == SampleFormat::F32 && config.channels() == 2
					})
					.expect("Host does not have output device that supports F32")
					.with_sample_rate(48000)
					.into();
				let stream = device.build_output_stream(
					&config,
					move |data: &mut [f32], _| {
						let available = if is_running() {
							match player_type {
								File => get_file_data(data),
								Wave => get_wave_data(data),
							}
						} else { false };
						if !available {
							data.fill(0.0);
							return;
						}
					},
					|_| {},
					Some(Duration::from_secs(5))
				).expect("Failed to create stream");
				// cpal will block when playing
				stream.play().unwrap();
			}
		}
		if *USE_PACAT {
			let mut pacat = get_pacat(player_type);
			if !pacat.discarded {
				pacat.child.kill().expect("Failed to kill pacat");
			}
		}
	});
}

fn get_file_data(buf: &mut [f32]) -> bool {
	let mut playing_files = acquire_playing_files();
	let mut eofs = vec![];
	if playing_files.len() > 0 {
		let volume = acquire().config.volume as f32 / 100.0;
		for (uuid, playable) in playing_files.iter_mut() {
			let max_read = buf.len().min(playable.data.len() - playable.position);
			for ii in 0..max_read {
				buf[ii] += playable.data[ii + playable.position] * linear_to_logarithmic(playable.volume * volume);
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
		return true;
	}
	false
}

fn get_wave_data(buf: &mut [f32]) -> bool {
	let mut playing_waves = acquire_playing_waves();
	if playing_waves.len() > 0 {
		let volume = acquire().config.volume as f32 / 100.0;
		for (_uuid, playable) in playing_waves.iter_mut() {
			let len = playable.len() as f32;
			let mut playable_bytes = [0_f32; CHUNK_SIZE];
			for wave in playable {
				for ii in 0..buf.len() / 2 {
					let sample = match wave.wave_type {
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
					} * wave.amplitude * linear_to_logarithmic(wave.volume * volume);
					playable_bytes[ii * 2] = sample;
					playable_bytes[ii * 2 + 1] = sample;
					wave.samples = (wave.samples + 1) % wave.period;
				}
			}
			for ii in 0..CHUNK_SIZE {
				buf[ii] += playable_bytes[ii] / len;
			}
		}
		return true;
	}
	false
}

fn linear_to_logarithmic(volume: f32) -> f32 {
	20.0 * volume.log10()
}