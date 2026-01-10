use std::{collections::{HashMap, HashSet}, sync::{Arc, LazyLock, Mutex, MutexGuard}, thread, time::Duration};

use mki::Keyboard;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{config::WaveformEntry, state::{acquire, notify_redraw}, util::global_input::keyboard_to_string};

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Default, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum WaveType {
	#[default]
	Sine,
	Square,
	Triangle,
	Saw
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Copy)]
pub struct Wave {
	pub wave_type: WaveType,
	pub frequency: f32,
	pub phase: f32,
	pub amplitude: f32,
}

impl Default for Wave {
	fn default() -> Self {
		Self {
			wave_type: WaveType::Sine,
			frequency: 1000.0,
			phase: 0.0, // percentage of the period
			amplitude: 1.0,
		}
	}
}

#[derive(Clone)]
pub struct Waveform {
	pub label: String,
	pub id: Option<u32>,
	pub keys: Vec<Keyboard>,
	pub waves: Vec<Wave>,
	pub volume: u32,
	pub playing: Arc<Mutex<(bool, bool)>>,
}

impl Default for Waveform {
	fn default() -> Self {
		Self {
			label: "New Waveform".to_string(),
			id: Option::None,
			keys: vec![],
			waves: vec![Wave::default()],
			volume: 100,
			playing: Arc::new(Mutex::new((false, false)))
		}
	}
}

impl Waveform {
	pub fn to_entry(&self) -> WaveformEntry {
		WaveformEntry {
			label: self.label.clone(),
			id: self.id,
			keys: self.keys.iter().map(|key| { keyboard_to_string(*key) }).collect::<HashSet<String>>(),
			waves: self.waves.clone(),
			volume: self.volume
		}
	}

	pub fn details(&self) -> String {
		if self.waves.len() == 1 {
			format!("{:?} {:.2} Hz", self.waves[0].wave_type,  self.waves[0].frequency)
		} else {
			format!("{:?} {:.2} Hz + {} more", self.waves[0].wave_type,  self.waves[0].frequency, self.waves.len() - 1)
		}
	}
}

pub struct PlayableWave {
	pub wave_type: WaveType,
	pub period: u32,
	pub samples: u32,
	pub amplitude: f32,
}

static PLAYING_WAVES: LazyLock<Mutex<HashMap<Uuid, Vec<PlayableWave>>>> = LazyLock::new(|| { Mutex::new(HashMap::new()) });

pub fn acquire_playing_waves() -> MutexGuard<'static, HashMap<Uuid, Vec<PlayableWave>>> {
	PLAYING_WAVES.lock().unwrap()
}

pub fn play_wave(wave: Waveform, auto_stop: bool) {
	thread::spawn(move || {
		let uuid = Uuid::new_v4();
		let mut app = acquire();

		if app.edit {
			return;
		}

		if wave.waves.len() == 0 {
			return;
		}

		let mut playing = wave.playing.lock().expect("Failed to check if wave is playing");
		if playing.0 {
			return;
		}
		playing.0 = true;
		drop(playing);

		let playable = wave.waves.iter().map(|w| {
			PlayableWave {
				wave_type: w.wave_type,
				period: (1.0 * 48000.0 / w.frequency) as u32,
				samples: (48000.0 * w.phase) as u32,
				amplitude: w.amplitude * (wave.volume as f32) / 100.0
			}
		}).collect::<Vec<PlayableWave>>();
		app.playing_wave.insert(uuid, format!("{} ({})", wave.label, wave.details()));
		acquire_playing_waves().insert(uuid, playable);
		drop(app);
		notify_redraw();

		if auto_stop {
			thread::sleep(Duration::from_secs(1));
		} else {
			while {
				let (playing, force) = *wave.playing.lock().unwrap();
				playing && force
			} || wave.keys.iter().all(|key| { key.is_pressed() }) {
				thread::sleep(Duration::from_millis(100));
			}
		}
		wave.playing.lock().unwrap().0 = false;

		acquire().playing_wave.remove(&uuid);
		acquire_playing_waves().remove(&uuid);
		notify_redraw();
	});
}

pub fn stop_all_waves() {
	// Defer to avoid deadlock
	thread::spawn(move || {
		let app = acquire();
		app.waves.iter().for_each(|wave| {
			let mut playing = wave.playing.lock().expect("Failed to lock mutex");
			playing.0 = false;
			playing.1 = false;
		});
	});
}