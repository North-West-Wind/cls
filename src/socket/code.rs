use std::{io::Write, os::unix::net::UnixStream, path::Path};

use clap::ArgMatches;

#[derive(PartialEq, Eq)]
pub enum SocketCode {
	Exit,
	ReloadConfig,
	AddTab,
	DeleteTab,
	ReloadTab,

	Play,
	PlayId,
	PlayWaveId,
	Stop,
	StopWaveId,

	SetVolume,
}

impl SocketCode {
	pub fn from_str(str: &str) -> Option<SocketCode> {
		use SocketCode::*;
		use Option::*;
		match str {
			"exit" => Some(Exit),
			"reload-config" => Some(ReloadConfig),
			"add-tab" => Some(AddTab),
			"delete-tab" => Some(DeleteTab),
			"reload-tab" => Some(ReloadTab),
			"play" => Some(Play),
			"play-id" => Some(PlayId),
			"play-wave" => Some(PlayWaveId),
			"stop" => Some(Stop),
			"stop-wave" => Some(StopWaveId),
			"set-volume" => Some(SetVolume),
			_ => None,
		}
	}

	pub fn from_u8(code: u8) -> Option<SocketCode> {
		use SocketCode::*;
		use Option::*;
		match code {
			1 => Some(Exit),
			2 => Some(ReloadConfig),
			3 => Some(AddTab),
			4 => Some(DeleteTab),
			5 => Some(ReloadTab),
			6 => Some(Play),
			9 => Some(PlayId),
			10 => Some(PlayWaveId),
			7 => Some(Stop),
			11 => Some(StopWaveId),
			8 => Some(SetVolume),
			_ => None,
		}
	}

	pub fn to_u8(&self) -> u8 {
		use SocketCode::*;
		match self {
			Exit => 1,
			ReloadConfig => 2,
			AddTab => 3,
			DeleteTab => 4,
			ReloadTab => 5,
			Play => 6,
			PlayId => 9,
			PlayWaveId => 10,
			Stop => 7,
			StopWaveId => 11,
			SetVolume => 8,
		}
	}

	pub fn write_to_stream(&self, mut stream: UnixStream, matches: &ArgMatches) -> std::io::Result<()> {
		use SocketCode::*;
		let mut buf = vec![self.to_u8()];
		match self {
			AddTab => {
				let path = matches.get_one::<String>("dir");
				buf.extend(path.expect("Missing `dir` argument").as_bytes());
			},
			DeleteTab|ReloadTab => {
				let index = matches.get_one::<String>("index");
				let path = matches.get_one::<String>("path");
				let name = matches.get_one::<String>("name");
				if index.is_some() {
					let index = index.unwrap().parse::<u8>();
					buf.extend([1, index.expect("Failed to parse index")]);
				} else if path.is_some() {
					let path = path.unwrap();
					if path.is_empty() {
						panic!("`path` must not be empty");
					}
					buf.push(2);
					buf.extend(path.as_bytes());

				} else if name.is_some() {
					let name = name.unwrap();
					if name.is_empty() {
						panic!("`name` must not be empty");
					}
					buf.push(3);
					buf.extend(name.as_bytes());
				} else {
					buf.push(0);
				}
			},
			Play => {
				let path = matches.get_one::<String>("path");
				buf.extend(path.expect("Missing `path` argument").as_bytes());
			},
			PlayId => {
				let id = matches.get_one::<String>("id").expect("Missing `id` argument").parse::<u32>();
				buf.extend(id.expect("Failed to parse ID").to_le_bytes());
			},
			PlayWaveId|StopWaveId => {
				let id = matches.get_one::<String>("id").expect("Missing `id` argument").parse::<u32>();
				buf.extend(id.expect("Failed to parse ID").to_le_bytes());
			},
			SetVolume => {
				let volume = matches.get_one::<String>("volume")
					.expect("Missing `volume` argument").parse::<i16>()
					.expect("Failed to parse volume");
				let increment = matches.get_flag("increment");
				if !increment && volume < 0 {
					panic!("Volume is negative, but it's not in increment mode");
				}
				if increment && (volume < -200 || volume > 200) {
					panic!("Volume increment can only be in range [-200, 200]");
				}
				buf.extend(volume.to_le_bytes());
				buf.push(if increment {1} else {0});

				let path = matches.get_one::<String>("path");
				if path.is_none() {
					buf.push(0);
				} else {
					let path = path.unwrap();
					let check_path = Path::new(path);
					if !check_path.is_file() {
						panic!("{} is not a file", path);
					}
					buf.push(1);
					buf.extend(path.as_bytes());
				}
			},
			_ => (),
		};
		stream.write_all(&buf)?;
		Ok(())
	}
}