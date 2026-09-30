use std::{collections::HashSet, time::SystemTime, vec};

use mki::Keyboard;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use serde::{Deserialize, Serialize};

use crate::common::keyboard::{keyboard_to_string, string_to_keyboard};

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
pub struct SingleWave {
	pub wave_type: WaveType,
	pub frequency: f32,
	pub phase: f32,
	pub amplitude: f32,
}

impl Default for SingleWave {
	fn default() -> Self {
		Self {
			wave_type: WaveType::Sine,
			frequency: 1000.0,
			phase: 0.0, // percentage of the period
			amplitude: 1.0,
		}
	}
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Default, Clone)]
pub struct SaveableWave {
	#[serde(default)]
	pub uid: u64,
	pub label: String,
	pub id: Option<u32>,
	pub keys: HashSet<String>,
	pub waves: Vec<SingleWave>,
	pub volume: u32,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct Wave {
	pub uid: u64,
	pub label: String,
	pub id: Option<u32>,
	pub keys: HashSet<Keyboard>,
	pub waves: Vec<SingleWave>,
	pub volume: u32,
}

impl Default for Wave {
	fn default() -> Self {
		Self {
			uid: SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_millis() as u64,
			label: "New Waveform".to_string(),
			id: None,
			keys: HashSet::new(),
			waves: vec![SingleWave::default()],
			volume: 100,
		}
	}
}

impl From<&SaveableWave> for Wave {
	fn from(wave: &SaveableWave) -> Self {
		Self {
			uid: wave.uid,
			label: wave.label.clone(),
			id: wave.id,
			keys: wave.keys.par_iter().filter_map(|key| string_to_keyboard(key)).collect::<HashSet<_>>(),
			waves: wave.waves.clone(),
			volume: wave.volume,
		}
	}
}

impl Into<SaveableWave> for Wave {
	fn into(self) -> SaveableWave {
		SaveableWave {
			uid: self.uid,
			label: self.label,
			id: self.id,
			keys: self.keys.par_iter().map(|key: &Keyboard| keyboard_to_string(*key)).collect::<HashSet<_>>(),
			waves: self.waves,
			volume: self.volume
		}
	}
}