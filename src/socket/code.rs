use std::{io::Write, os::unix::net::UnixStream};

use clap::ArgMatches;

pub enum SocketCode {
	Exit,
	ReloadConfig,
	AddTab,
	DeleteCurrentTab,
	ReloadCurrentTab,
}

impl SocketCode {
	pub fn from_str(str: &str) -> Option<SocketCode> {
		use SocketCode::*;
		use Option::*;
		match str {
			"exit" => Some(Exit),
			"reload-config" => Some(ReloadConfig),
			"add-tab" => Some(AddTab),
			"delete-current-tab" => Some(DeleteCurrentTab),
			"reload-current-tab" => Some(ReloadCurrentTab),
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
			4 => Some(DeleteCurrentTab),
			5 => Some(ReloadCurrentTab),
			_ => None,
		}
	}

	pub fn to_u8(&self) -> u8 {
		use SocketCode::*;
		match self {
			Exit => 1,
			ReloadConfig => 2,
			AddTab => 3,
			DeleteCurrentTab => 4,
			ReloadCurrentTab => 5,
		}
	}

	pub fn write_to_stream(&self, mut stream: UnixStream, matches: &ArgMatches) -> std::io::Result<()> {
		use SocketCode::*;
		let code = self.to_u8();
		stream.write(&[code])?;
		match self {
			AddTab => {
				let path = matches.get_one::<String>("path");
				if path.is_none() {
					stream.write_all(b"")?;
				} else {
					stream.write_all(path.unwrap().as_bytes())?;
				}
			},
			_ => (),
		};
		Ok(())
	}
}