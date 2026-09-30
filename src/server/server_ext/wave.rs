use std::{thread, time::Duration};

use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

use crate::{common::base::wave::{Wave, WaveType}, server::AtomicServerState};

pub struct PlayableWave {
	pub wave_type: WaveType,
	pub period: f32,
	pub phase: f32,
	pub amplitude: f32,
	pub volume: f32,
}

#[derive(Clone, Default)]
pub struct ServerWave {
	pub base: Wave,
	pub forced: bool,
}

impl ServerWave {
	pub fn play(&self, server_state: AtomicServerState, auto_stop: bool) {
		let uid = self.base.uid;
		if self.base.waves.len() == 0 || { server_state.read().playable_waves.contains_key(&uid) } {
			return;
		}

		let playable = self.base.waves.par_iter().map(|w| {
			PlayableWave {
				wave_type: w.wave_type,
				period: 1.0 / w.frequency,
				phase: w.phase / w.frequency,
				amplitude: w.amplitude,
				volume: self.base.volume as f32 / 100.0
			}
		}).collect::<Vec<PlayableWave>>();
		{ server_state.write().playable_waves.insert(uid, playable); }

		if auto_stop {
			thread::sleep(Duration::from_secs(1));
		} else {
			while self.forced || { server_state.read().playable_waves.contains_key(&uid) } || self.base.keys.par_iter().all(|key| { key.is_pressed() }) {
				thread::sleep(Duration::from_millis(100));
			}
		}

		{ server_state.write().playable_waves.remove(&uid); }
	}
}