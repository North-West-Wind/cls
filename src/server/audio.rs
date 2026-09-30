use std::{collections::HashMap, f32::consts::PI, format, io::{self, BufWriter, Write}, process::{Child, ChildStdin, Command, Stdio}, str::FromStr, sync::LazyLock, thread, time::{Duration, SystemTime}, vec};

use cmd_exists::cmd_exists;
use cpal::{DeviceId, SampleFormat, traits::{DeviceTrait, HostTrait, StreamTrait}};
use parking_lot::{Mutex, MutexGuard};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use uuid::Uuid;

use crate::{common::{base::wave::WaveType, constant::{APP_NAME, ENDIANESS}, log}, server::{AtomicServerState, file::PlayableFile, server_ext::wave::PlayableWave}};

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

fn spawn_pacat(player_type: PlayerType, sample_rate: u32) -> Pacat {
	use PlayerType::*;
	let channels: u8 = match player_type {
		File => 2,
		Wave => 2,
	};
	let mut child = Command::new("pacat").args([
		"-d",
		APP_NAME,
		format!("--channels={}", channels).as_str(),
		format!("--rate={}", sample_rate).as_str(),
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
	static FILES: LazyLock<Mutex<Pacat>> = LazyLock::new(|| Mutex::new(spawn_pacat(PlayerType::File, 48000)));
	static WAVES: LazyLock<Mutex<Pacat>> = LazyLock::new(|| Mutex::new(spawn_pacat(PlayerType::Wave, 48000)));
	use PlayerType::*;
	match player_type {
		File => FILES.lock(),
		Wave => WAVES.lock()
	}
}

pub fn create_audio_player(atomic_server_state: AtomicServerState, player_type: PlayerType) {
	use PlayerType::*;
	let server_state = atomic_server_state.clone();
	let mut server_state = server_state.write();
	if server_state.no_pacat || cmd_exists("pacat").is_err() {
		// No pacat. Use cpal
		let target_device = server_state.cpal_device.clone();
		let device = if target_device.is_empty() {
			cpal::default_host().default_output_device().expect("Failed to get default output device")
		} else {
			let device_id = DeviceId::from_str(&target_device).expect("Failed to create device ID");
			let host = cpal::host_from_id(device_id.0).expect("Failed to get host");
			host.device_by_id(&device_id).expect("Failed to find target device")
		};
		let (sample_rate, config) = {
			let mut config_range = device.supported_output_configs().unwrap().cycle();
			let config = config_range
			.find(|config| {
				config.sample_format() == SampleFormat::F32 && config.channels() == 2
			})
			.unwrap_or(config_range.next().expect("No output device"));
			let sample_rate = config.max_sample_rate().min(48000);
			let config = config.with_sample_rate(sample_rate);
			(sample_rate, config.into())
		};
		server_state.sample_rate = sample_rate;
		log::info(format!("Sample rate: {}", sample_rate));
		let err_callback = |err| {
			log::error(format!("{:?}", err));
		};
		use PlayerType::*;
		let server_state = atomic_server_state.clone();
		let stream = match player_type {
			File => {
				device.build_output_stream(&config, move |data: &mut [f32], _| {
					let mut playable_files = &mut atomic_server_state.write().playable_files;
					let mut buf = vec![0.0; data.len()];
					get_file_data(&mut buf, server_state.read().config.volume, &mut playable_files);
					data.copy_from_slice(&buf);
				}, err_callback, None)
			},
			Wave => {
				device.build_output_stream(&config, move |data: &mut [f32], _| {
					let mut playable_waves = &mut atomic_server_state.write().playable_waves;
					let mut buf = vec![0.0; data.len()];
					get_wave_data(&mut buf, sample_rate, server_state.read().config.volume, &mut playable_waves);
					data.copy_from_slice(&buf);

				}, err_callback, None)
			}
		}.expect("Failed to create stream");
		stream.play().unwrap();
	} else {
		let server_state = atomic_server_state.clone();
		thread::spawn(move || {
			let mut buf = [0_f32; CHUNK_SIZE];
			while server_state.read().running {
				let (sample_rate, available) = {
					let server_state = &mut server_state.write();
					let sample_rate = server_state.sample_rate;
					let volume = server_state.config.volume;
					let available = match player_type {
						File => get_file_data(&mut buf, volume, &mut server_state.playable_files),
						Wave => get_wave_data(&mut buf, sample_rate, volume, &mut server_state.playable_waves),
					};
					(sample_rate, available)
				};
				let mut pacat = get_pacat(player_type);
				if available {
					if pacat.discarded {
						// Pacat is killed. Needs respawn
						*pacat = spawn_pacat(player_type, sample_rate);
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
			}
			let mut pacat = get_pacat(player_type);
			if !pacat.discarded {
				pacat.child.kill().expect("Failed to kill pacat");
			}
		});
	}
}

fn get_file_data(buf: &mut [f32], volume: u32, playable_files: &mut HashMap<Uuid, PlayableFile>) -> bool {
	if playable_files.len() > 0 {
		// No parallel because it creates too much overhead
		let volume = volume as f32 / 100.0;
		for (_uuid, playable) in playable_files.iter_mut() {
			let volume = linear_to_logarithmic(playable.volume * volume);
			let max_read = buf.len().min(playable.data.len() - playable.position);
			for ii in 0..max_read {
				buf[ii] += playable.data[ii + playable.position] * volume;
			}
			playable.position += max_read;
		}
		let eofs = playable_files.par_iter().filter_map(|(uuid, playable)| {
			if playable.position == playable.data.len() {
				let (lock, cvar) = &*playable.finished;
				let _locked = lock.lock();
				cvar.notify_one();
				Some(*uuid)
			} else {
				None
			}
		}).collect::<Vec<_>>();
		if !eofs.is_empty() {
			eofs.iter().for_each(|uuid| {
				playable_files.remove(uuid);
			});
		}
		return true;
	}
	false
}

fn get_wave_data(buf: &mut [f32], sample_rate: u32, volume: u32, playable_waves: &mut HashMap<u64, Vec<PlayableWave>>) -> bool {
	if playable_waves.len() > 0 {
		let volume = volume as f32 / 100.0;
		// No parallel because it creates too much overhead
		for (_uuid, playable) in playable_waves.iter_mut() {
			let len = playable.len() as f32;
			let mut playable_bytes = vec![0_f32; buf.len()];
			for wave in playable {
				for ii in 0..buf.len() / 2 {
					let sample = match wave.wave_type {
						WaveType::Sine => (PI * 2.0 * wave.phase).sin(),
						WaveType::Square => if wave.phase > 0.5 { 1.0 } else { -1.0 },
						WaveType::Triangle => {
							let portion = wave.phase;
							if portion > 0.5 {
								-1.0 + (portion - 0.5) * 4.0
							} else {
								1.0 - portion * 4.0
							}
						},
						WaveType::Saw => -1.0 + wave.phase * 2.0,
					} * wave.amplitude * linear_to_logarithmic(wave.volume * volume);
					playable_bytes[ii * 2] += sample;
					playable_bytes[ii * 2 + 1] += sample;
					wave.phase = wave.phase + (1.0 / sample_rate as f32) / wave.period;
					if wave.phase >= 1.0 {
						wave.phase -= 1.0;
					}
				}
			}
			for ii in 0..playable_bytes.len() {
				buf[ii] += playable_bytes[ii] / len;
			}
		}
		return true;
	}
	false
}

fn linear_to_logarithmic(volume: f32) -> f32 {
	if volume <= 0.0 {
		0.0
	} else {
		0.001 * (1000_f32).powf(volume)
	}
}