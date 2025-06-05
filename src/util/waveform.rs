use std::{collections::HashSet, f32::consts::PI, io::Write, process::{Command, Stdio}, sync::{Arc, Mutex}, thread, time::Duration};

use mki::Keyboard;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{config::WaveformEntry, constant::APP_NAME, state::get_mut_app, util::{global_input::keyboard_to_string, notify_redraw}};

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
	pub playing: Arc<Mutex<bool>>,
}

impl Default for Waveform {
	fn default() -> Self {
		Self {
			label: "New Waveform".to_string(),
			id: Option::None,
			keys: vec![],
			waves: vec![Wave::default()],
			volume: 50,
			playing: Arc::new(Mutex::new(false))
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
}

struct PlayableWave {
	pub wave_type: WaveType,
	pub period: u32,
	pub samples: u32,
	pub amplitude: f32,
}

pub fn play_wave(wave: Waveform, auto_stop: bool) {
	thread::spawn(move || {
		let uuid = Uuid::new_v4();
		let app = get_mut_app();

		if app.edit {
			app.playing_wave.insert(uuid, (0, "Edit-only mode!".to_string()));
			notify_redraw();
			thread::sleep(Duration::from_secs(1));
			app.playing_wave.remove(&uuid);
			notify_redraw();
			return;
		}

		if wave.waves.len() == 0 {
			return;
		}

		let mut playing = wave.playing.lock().expect("Failed to check if wave is playing");
		if *playing {
			return;
		}
		*playing = true;
		std::mem::drop(playing);

		let volume = (wave.volume * 65535 / 100) as u16;

		let mut pacat_child = Command::new("pacat").args([
			"-d",
			APP_NAME,
			"--channels=1",
			"--rate=48000",
			"--format=float32le",
			format!("--volume={}", volume).as_str(),
		])
			.stdin(Stdio::piped())
			.stdout(Stdio::piped()).spawn().expect("Failed to spawn pacat process");

		let mut stdin = pacat_child.stdin.take().expect("Failed to obtain pacat stdin");
		let mutex = wave.playing.clone();
		thread::spawn(move || {
			let mut lock = mutex.lock().expect("Failed to get shared mutex");
			let mut playing = *lock;
			std::mem::drop(lock);
			let mut playable = wave.waves.iter().map(|w| {
				PlayableWave {
					wave_type: w.wave_type,
					period: (1.0 * 48000.0 / w.frequency) as u32,
					samples: (48000.0 * w.phase) as u32,
					amplitude: w.amplitude
				}
			}).collect::<Vec<PlayableWave>>();
			while playing {
				let mut sum_bytes = [0_f32; 1600];
				for wave in &mut playable {
					for ii in 0..1600 {
						sum_bytes[ii] += match wave.wave_type {
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
				// Average it out
				let mut bytes = [0_u8; 6400];
				let len = playable.len() as i32;
				for ii in 0..1600 {
					[
						bytes[ii * 4],
						bytes[ii * 4 + 1],
						bytes[ii * 4 + 2],
						bytes[ii * 4 + 3]
					] = ((sum_bytes[ii] / len as f32)).to_le_bytes();
				}
				// Write bytes to stdin
				stdin.write_all(&bytes).expect("Failed to write to pacat stdin");
				// Wait to write next chunk
				thread::sleep(Duration::from_secs_f32(1.0 / (48000.0 / 1600.0)));
				lock = mutex.lock().expect("Failed to get shared mutex");
				playing = *lock;
				std::mem::drop(lock);
			}
		});

		app.playing_wave.insert(uuid, (pacat_child.id(), wave.label));
		notify_redraw();

		if auto_stop {
			thread::sleep(Duration::from_secs(1));
			let mutex = wave.playing.clone();
			let mut playing = mutex.lock().expect("Failed to lock mutex");
			*playing = false;
			std::mem::drop(playing);
		}
		
		pacat_child.wait().expect("Failed to wait for pacat");
		app.playing_wave.remove(&uuid);
		notify_redraw();
	});
}

pub fn stop_all_waves() {
	let app = get_mut_app();
	app.waves.iter().for_each(|wave| {
		let mut playing = wave.playing.lock().expect("Failed to lock mutex");
		*playing = false;
	});
}