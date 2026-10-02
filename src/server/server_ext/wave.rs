use std::{f32::consts::PI, sync::{Arc, atomic::{AtomicBool, Ordering}}, thread::{self, JoinHandle}, time::Duration, vec};

use parking_lot::Mutex;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
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
	pub forced: Arc<AtomicBool>,
	pub playing: Arc<AtomicBool>,
}

impl From<Wave> for ServerWave {
	fn from(base: Wave) -> Self {
		Self {
			base,
			forced: Arc::new(AtomicBool::new(false)),
			playing: Arc::new(AtomicBool::new(false)),
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

		let forced = self.forced.clone();
		let playing = self.playing.clone();
		let keys = self.base.keys.clone();
		Some(thread::spawn(move || {
			playing.store(true, Ordering::Relaxed);
			let mut buf = vec![0f32; sample_rate / 16];
			while forced.load(Ordering::Relaxed) || keys.par_iter().all(|key| key.is_pressed()) {
				for wave in playable.iter_mut() {
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
						} * wave.amplitude * wave.volume;
						buf[ii * 2] += sample;
						buf[ii * 2 + 1] += sample;
						wave.phase = wave.phase + (1.0 / sample_rate as f32) / wave.period;
						if wave.phase >= 1.0 {
							wave.phase -= 1.0;
						}
					}
				}

				let mut offset = prod.push_slice(&buf);
				while offset < buf.len() {
					thread::sleep(Duration::from_millis(10));
					offset += prod.push_slice(&buf[offset..]);
				}
				buf.fill(0.0);
			}
			playing.store(false, Ordering::Relaxed);
		}))
	}
}