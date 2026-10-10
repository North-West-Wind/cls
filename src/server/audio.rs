use std::{format, io::{self, BufWriter, Write}, process::{Child, ChildStdin, Command, Stdio}, str::FromStr, sync::{Arc, mpsc::Receiver}, thread, time::{Duration, SystemTime}, vec};

use cmd_exists::cmd_exists;
use cpal::{DeviceId, SampleFormat, traits::{DeviceTrait, HostTrait, StreamTrait}};
use parking_lot::RwLock;
use rayon::iter::{IndexedParallelIterator, IntoParallelRefIterator, IntoParallelRefMutIterator, ParallelIterator};
use ringbuf::{HeapCons, traits::Consumer};

use crate::{common::{constant::{APP_NAME, ENDIANESS}, log}, server::{AudioSettings, State}};

const CHUNK_SIZE: usize = 1024;

struct Pacat {
	last_used: SystemTime,
	child: Child,
	writer: BufWriter<ChildStdin>,
}

fn spawn_pacat(sample_rate: u32) -> Pacat {
	let mut child = Command::new("pacat").args([
		"-d",
		APP_NAME,
		"--channels=2",
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
	}
}

pub fn create_audio_player(state: Arc<RwLock<State>>, audio_settings: Arc<RwLock<AudioSettings>>, rx: Receiver<HeapCons<f32>>) {
	if audio_settings.read().no_pacat || cmd_exists("pacat").is_err() {
		// No pacat. Use cpal
		let target_device = audio_settings.read().cpal_device.clone();
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
		audio_settings.write().sample_rate = sample_rate;
		log::info(format!("Sample rate: {}", sample_rate));
		let err_callback = |err| {
			log::error(err);
		};
		let mut audio_data = vec![];
		let stream = device.build_output_stream(&config, move |data: &mut [f32], _| {
			let volume = state.read().config.volume;
			if let Ok(cons) = rx.try_recv() {
				audio_data.push((cons, false));
			}
			read_samples(data, volume, &mut audio_data);
		}, err_callback, None).expect("Failed to create stream");
		stream.play().unwrap();
	} else {
		thread::spawn(move || {
			let mut pacat_holder: Option<Pacat> = None;
			let mut audio_data = vec![];
			let mut buf = [0_f32; CHUNK_SIZE];
			while state.read().running {
				if let Ok(cons) = rx.try_recv() {
					audio_data.push((cons, false));
				}
				let (sample_rate, available) = {
					let sample_rate = audio_settings.read().sample_rate;
					let volume = state.read().config.volume;
					let available = read_samples(&mut buf, volume, &mut audio_data);
					(sample_rate, available)
				};
				if available {
					let mut pacat = if let Some(pacat) = pacat_holder {
						pacat
					} else {
						spawn_pacat(sample_rate)
					};

					pacat.writer.write_all(&bytemuck::cast_slice(&buf)).expect("Failed to write to pacat stdin");
					// If blocked, we wait
					loop {
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
					pacat_holder = Some(pacat);
				} else if let Some(ref mut pacat) = pacat_holder && SystemTime::now().duration_since(pacat.last_used).unwrap().as_secs() > 5 {
					// Kill pacat when not used
					pacat.child.kill().expect("Failed to kill pacat");
					pacat_holder = None;
				}
				thread::sleep(Duration::from_millis(10));
			}

			if let Some(ref mut pacat) = pacat_holder {
				pacat.child.kill().expect("Failed to kill pacat");
			}
		});
	}
}

fn read_samples(buf: &mut [f32], volume: u32, audio_data: &mut Vec<(HeapCons<f32>, bool)>) -> bool {
	if audio_data.is_empty() {
		return false;
	}
	let volume = volume as f32 / 100.0;
	let mut in_buf = vec![0f32; buf.len()];
	// Read data from ring buffers & remove inactive
	audio_data.retain_mut(|(consumer, active)| {
		let read = consumer.pop_slice(&mut in_buf);
		if read > 0 {
			*active = true;
			buf[..read].par_iter_mut().zip(in_buf[..read].par_iter()).for_each(|(dst, src)| *dst += src * volume);
			true
		} else if *active {
			false
		} else {
			true
		}
	});
	true
}