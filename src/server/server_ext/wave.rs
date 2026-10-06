use std::{f32::consts::PI, sync::{Arc, atomic::{AtomicBool, Ordering}}, thread::{self, JoinHandle}, time::Duration, vec};

use parking_lot::Mutex;
use rayon::{iter::{IndexedParallelIterator, IntoParallelRefIterator, IntoParallelRefMutIterator, ParallelIterator}, slice::ParallelSliceMut};
use ringbuf::{HeapRb, traits::{Producer, Split}};
use uuid::Uuid;

use crate::{common::base::wave::{Wave, WaveType}, server::AtomicServerState};

struct PlayableWave {
	pub wave_type: WaveType,
	pub period: f32,
	pub phase: f32,
	pub amplitude: f32,
	pub volume: f32,
}

#[derive(Clone, Default)]
pub struct ServerWave {
	pub base: Wave,
	pub active: Arc<AtomicBool>,
}

impl From<Wave> for ServerWave {
	fn from(base: Wave) -> Self {
		Self {
			base,
			active: Arc::new(AtomicBool::new(false)),
		}
	}
}

impl ServerWave {
	pub fn play(&self, server_state: AtomicServerState) -> Option<JoinHandle<()>> {
		if self.base.waves.len() == 0 {
			return None;
		}

		let mut playable = self.base.waves.par_iter().map(|w| {
			PlayableWave {
				wave_type: w.wave_type,
				period: 1.0 / w.frequency,
				phase: w.phase / w.frequency,
				amplitude: w.amplitude,
				volume: self.base.volume as f32 / 100.0
			}
		}).collect::<Vec<PlayableWave>>();

		let uuid = Uuid::new_v4();
		let (sample_rate, mut prod) = {
			let mut server_state = server_state.write();
			let sample_rate = server_state.sample_rate as usize;
			let rb = HeapRb::<f32>::new(sample_rate / 16);
			let (prod, cons) = rb.split();
			server_state.audio_data.insert(uuid, Arc::new(Mutex::new(cons)));
			(sample_rate, prod)
		};

		let active = self.active.clone();
		Some(thread::spawn(move || {
			let mut buf = vec![0f32; sample_rate / 16];
			while active.load(Ordering::Relaxed) {
				buf.par_chunks_exact_mut(2).enumerate().for_each(|(ii, samples)| {
					let sample = playable.par_iter().map(|wave| {
						let mut phase = wave.phase + ((1.0 + ii as f32) / sample_rate as f32) / wave.period;
						while phase >= 1.0 {
							phase -= 1.0;
						}
						let sample = match wave.wave_type {
							WaveType::Sine => (PI * 2.0 * phase).sin(),
							WaveType::Square => if phase > 0.5 { 1.0 } else { -1.0 },
							WaveType::Triangle => {
								let portion = phase;
								if portion > 0.5 {
									-1.0 + (portion - 0.5) * 4.0
								} else {
									1.0 - portion * 4.0
								}
							},
							WaveType::Saw => -1.0 + phase * 2.0,
						} * wave.amplitude * wave.volume;
						sample
					}).sum::<f32>();
					samples[0] = sample;
					samples[1] = sample;
				});
				let phase_delta = (buf.len() / 2) as f32;
				playable.par_iter_mut().for_each(|wave| {
					wave.phase = wave.phase + (phase_delta / sample_rate as f32) / wave.period;
					while wave.phase >= 1.0 {
						wave.phase -= 1.0;
					}
				});

				let mut offset = prod.push_slice(&buf);
				while offset < buf.len() {
					thread::sleep(Duration::from_millis(10));
					offset += prod.push_slice(&buf[offset..]);
				}
				buf.fill(0.0);
			}
		}))
	}
}