use std::format;

use crate::common::base::wave::Wave;

#[derive(Debug, Default, Clone)]
pub struct ClientWave {
	pub base: Wave
}

impl From<Wave> for ClientWave {
	fn from(value: Wave) -> Self {
		Self { base: value }
	}
}

impl ClientWave {
	pub fn details(&self) -> String {
		if self.base.waves.len() == 1 {
			format!("{:?} {:.2} Hz", self.base.waves[0].wave_type,  self.base.waves[0].frequency)
		} else {
			format!("{:?} {:.2} Hz + {} more", self.base.waves[0].wave_type,  self.base.waves[0].frequency, self.base.waves.len() - 1)
		}
	}
}