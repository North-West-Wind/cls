use std::{io::{Read, Write}, path::Path};

use clap::ArgMatches;
use interprocess::local_socket::Stream;

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

	pub fn write_to_stream(&self, mut stream: Stream, matches: &ArgMatches) -> std::io::Result<String> {
		use SocketCode::*;
		let mut buf = vec![self.to_u8()];
		match self {
			AddTab => {
				let path = matches.get_one::<String>("dir");
				buf.extend(path.expect("Missing `dir` argument").as_bytes());
				stream.write_all(&buf)?;
				let mut res = [0u8; 256];
				stream.read(&mut res)?;
				return match res[0] {
					0 => {
						let path = String::from_utf8(res[1..256].to_vec());
						Ok(format!("Success\n{}", path.map_or("Path unknown".to_string(), |path| {
							format!("Added {}", path)
						})))
					},
					1 => Ok("Failed\nPath is empty".to_string()),
					2 => Ok("Failed\nPath does not exist or cannot be accessed".to_string()),
					_ => Ok("Failed\nResponse code is unknown".to_string())
				}
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
				stream.write_all(&buf)?;
				let mut res = [0u8; 256];
				stream.read(&mut res)?;
				return match res[0] {
					0|10 => {
						let path = String::from_utf8(res[1..256].to_vec());
						Ok(format!("Success\n{}", path.map_or("Path unknown".to_string(), |path| {
							format!("{} {}", if res[0] == 0 { "Deleted " } else { "Reloaded " }, path)
						})))
					},
					1 => Ok("Failed\nPath does not exist or cannot be accessed".to_string()),
					2 => Ok("Failed\nPath is a tab".to_string()),
					3 => Ok("Failed\nCould not find by name".to_string()),
					4 => Ok("Failed\nIndex out of range".to_string()),
					_ => Ok("Failed\nResponse code is unknown".to_string())
				}
			},
			Play => {
				let path = matches.get_one::<String>("path");
				buf.extend(path.expect("Missing `path` argument").as_bytes());
				stream.write_all(&buf)?;
				let mut res = [0u8; 256];
				stream.read(&mut res)?;
				return match res[0] {
					0 => {
						let path = String::from_utf8(res[1..256].to_vec());
						Ok(format!("Success\n{}", path.map_or("Path unknown".to_string(), |path| {
							format!("Playing {}", path)
						})))
					},
					1 => Ok("Failed\nPath is empty".to_string()),
					_ => Ok("Failed\nResponse code is unknown".to_string())
				}
			},
			PlayId|PlayWaveId|StopWaveId => {
				let id = matches.get_one::<String>("id").expect("Missing `id` argument").parse::<u32>().expect("Failed to parse ID");
				buf.extend(id.to_le_bytes());
				stream.write_all(&buf)?;
				let mut res = [0u8; 256];
				stream.read(&mut res)?;
				return match res[0] {
					0|10 => {
						let label = String::from_utf8(res[1..256].to_vec());
						Ok(format!("Success\n{}", label.map_or("Path unknown".to_string(), |path| {
							format!("{} {}", if res[0] == 0 { "Playing" } else { "Stopping" }, path)
						})))
					},
					1 => Ok(format!("Failed\nID {} does not exist", id)),
					_ => Ok("Failed\nResponse code is unknown".to_string())
				}
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
				stream.write_all(&buf)?;
				let mut res = [0u8; 5];
				stream.read(&mut res)?;
				return match res[0] {
					0 => {
						let new_volume = u32::from_le_bytes([res[1], res[2], res[3], res[4]]);
						Ok(format!("Success\nNew volume: {}", new_volume))
					},
					1 => Ok("Failed\nFile does not exist".to_string()),
					_ => Ok("Failed\nResponse code is unknown".to_string())
				}
			},
			_ => {
				stream.write_all(&buf)?;
				let mut res = [0u8; 256];
				stream.read(&mut res)?;
				return match res[0] {
					0|10 => Ok("Success".to_string()),
					_ => Ok("Failed\nResponse code is unknown".to_string())
				}
			},
		};
	}
}