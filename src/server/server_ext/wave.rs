use std::{f32::consts::PI, sync::{Arc, atomic::{AtomicBool, Ordering}, mpsc::Sender}, thread::{self, JoinHandle}, time::Duration, vec};

use rayon::{iter::{IndexedParallelIterator, IntoParallelRefIterator, IntoParallelRefMutIterator, ParallelIterator}, slice::ParallelSliceMut};
use ringbuf::{HeapCons, HeapRb, traits::{Producer, Split}};

use crate::{common::base::wave::{SaveableWave, WaveType}};

struct PlayableWave {
	pub wave_type: WaveType,
	pub period: f32,
	pub phase: f32,
	pub amplitude: f32,
	pub volume: f32,
}

#[derive(Clone, Default)]
pub struct ServerWave {
	pub base: SaveableWave,
	pub active: Arc<AtomicBool>,
}

impl From<SaveableWave> for ServerWave {
	fn from(base: SaveableWave) -> Self {
		Self {
			base,
			active: Arc::new(AtomicBool::new(false)),
		}
	}
}

impl ServerWave {
	pub fn play(&self, sample_rate: u32, audio_sender: &mut Sender<HeapCons<f32>>) -> Option<JoinHandle<()>> {
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

		let mut prod = {
			let rb = HeapRb::<f32>::new(sample_rate as usize / 16);
			let (prod, cons) = rb.split();
			audio_sender.send(cons).unwrap();
			prod
		};

		let active = self.active.clone();
		Some(thread::spawn(move || {
			let mut buf = vec![0f32; sample_rate as usize / 16];
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