use std::{fmt::{self, Debug, Display}, io::{self, ErrorKind, Read}, net::TcpStream, vec, write};

use serde::Serialize;

use crate::common::base::{dialog::SaveableDialog, file::SaveableFile, wave::SaveableWave};

#[derive(Clone)]
struct UnknownMsgTypeError {
	msg_type: u8
}

impl std::error::Error for UnknownMsgTypeError {}

impl Debug for UnknownMsgTypeError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "Unknown message type: {}", self.msg_type)
	}
}

impl Display for UnknownMsgTypeError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "Unknown message type: {}", self.msg_type)
	}
}

impl UnknownMsgTypeError {
	fn new(msg_type: u8) -> Self {
		Self { msg_type }
	}
}

#[derive(Debug, PartialEq)]
pub enum ClientToServer {
	// Starts from 1
	Exit,
	Reload,

	// Starts from 11
	PlayPath(String),
	PlayWave(u64),
	PlayDialog(u64),
	PlaySearch(String),
	StopFiles,
	StopWave(u64),
	StopDialog(u64),

	// Starts from 21
	SetLoopbacks(Vec<String>),
	SetSinkVolume(u32),
	SetFile(String, SaveableFile),
	SetWave(u64, SaveableWave),
	SetDialog(u64, SaveableDialog),
	DeleteWave(u64),
	DeleteDialog(u64),
	SetStopKey(Vec<String>),
	SetPlaylistMode(bool),
}

#[derive(Debug, PartialEq)]
pub enum ServerToClient {
	// Starts from 1
	Success,
	Error(String),
	Reload,

	// Starts from 11
	Playing(u8, u16, String),
	Stopping(u16)
}

pub fn decode_c2s(msg: &[u8]) -> Result<ClientToServer, Box<dyn std::error::Error>> {
	use ClientToServer::*;
	let result = match msg[0] {
		1 => Ok(Exit),
		2 => Ok(Reload),
		11 => {
			let path = String::from_utf8(msg[1..].try_into()?)?;
			Ok(PlayPath(path))
		},
		12 => {
			let id = u64::from_be_bytes(msg[1..].try_into()?);
			Ok(PlayWave(id))
		},
		13 => {
			let id = u64::from_be_bytes(msg[1..].try_into()?);
			Ok(PlayDialog(id))
		},
		14 => {
			let query = String::from_utf8(msg[1..].try_into()?)?;
			Ok(PlaySearch(query))
		},
		15 => Ok(StopFiles),
		16 => {
			let id = u64::from_be_bytes(msg[1..].try_into()?);
			Ok(StopWave(id))
		},
		17 => {
			let id = u64::from_be_bytes(msg[1..].try_into()?);
			Ok(StopDialog(id))
		},
		21 => {
			let mut offset = 5;
			let mut loopbacks = vec![];
			for _ in 0..u32::from_be_bytes(msg[1..5].try_into()?) as usize {
				let len = u32::from_be_bytes(msg[offset..(offset + 4)].try_into()?) as usize;
				loopbacks.push(String::from_utf8(msg[(offset + 4)..(offset + 4 + len)].try_into()?)?);
				offset += 4 + len;
			}
			Ok(SetLoopbacks(loopbacks))
		},
		22 => {
			let volume = u32::from_be_bytes(msg[1..].try_into()?);
			Ok(SetSinkVolume(volume))
		},
		23 => {
			let path_length = u32::from_be_bytes(msg[1..5].try_into()?) as usize;
			let path = String::from_utf8(msg[5..(path_length + 5)].try_into()?)?;
			let file = rmp_serde::from_slice(&msg[(path_length + 5)..])?;
			Ok(SetFile(path, file))
		},
		24 => {
			let uid = u64::from_be_bytes(msg[1..9].try_into()?);
			let wave = rmp_serde::from_slice(&msg[9..])?;
			Ok(SetWave(uid, wave))
		},
		25 => {
			let uid = u64::from_be_bytes(msg[1..9].try_into()?);
			let dialog = rmp_serde::from_slice(&msg[9..])?;
			Ok(SetDialog(uid, dialog))
		},
		26 => {
			let id = u64::from_be_bytes(msg[1..].try_into()?);
			Ok(DeleteWave(id))
		},
		27 => {
			let id = u64::from_be_bytes(msg[1..].try_into()?);
			Ok(DeleteDialog(id))
		},
		28 => {
			let mut offset = 5;
			let mut keys = vec![];
			for _ in 0..u32::from_be_bytes(msg[1..5].try_into()?) as usize {
				let len = u32::from_be_bytes(msg[offset..(offset + 4)].try_into()?) as usize;
				keys.push(String::from_utf8(msg[(offset + 4)..(offset + 4 + len)].try_into()?)?);
				offset += 4 + len;
			}
			Ok(SetStopKey(keys))
		},
		29 => Ok(SetPlaylistMode(msg[1] != 0)),
		_ => Err(UnknownMsgTypeError::new(msg[0]))
	}?;
	Ok(result)
}

pub fn decode_s2c(msg: &[u8]) -> Result<ServerToClient, Box<dyn std::error::Error>> {
	use ServerToClient::*;
	let result = match msg[0] {
		1 => Ok(Success),
		2 => {
			let message = String::from_utf8(msg[1..].try_into()?)?;
			Ok(Error(message))
		},
		3 => Ok(Reload),
		11 => {
			let id = u16::from_be_bytes(msg[2..4].try_into()?);
			let body = String::from_utf8(msg[4..].try_into()?)?;
			Ok(Playing(msg[1], id, body))
		},
		12 => {
			let id = u16::from_be_bytes(msg[1..].try_into()?);
			Ok(Stopping(id))
		},
		_ => Err(UnknownMsgTypeError::new(msg[0]))
	}?;
	Ok(result)
}

pub fn encode_c2s(request: &ClientToServer) -> Vec<u8> {
	use ClientToServer::*;
	match request {
		Exit => vec![1u8],
		Reload => vec![2u8],
		PlayPath(path) => {
			let mut buf = vec![11u8];
			buf.extend_from_slice(path.as_bytes());
			buf
		},
		PlayWave(uid) => {
			let mut buf = vec![12u8];
			buf.extend(uid.to_be_bytes());
			buf
		},
		PlayDialog(uid) => {
			let mut buf = vec![13u8];
			buf.extend(uid.to_be_bytes());
			buf
		},
		PlaySearch(query) => {
			let mut buf = vec![14u8];
			buf.extend_from_slice(query.as_bytes());
			buf
		},
		StopFiles => vec![15u8],
		StopWave(uid) => {
			let mut buf = vec![16u8];
			buf.extend(uid.to_be_bytes());
			buf
		},
		StopDialog(uid) => {
			let mut buf = vec![17u8];
			buf.extend(uid.to_be_bytes());
			buf
		},
		SetLoopbacks(loopbacks) => {
			let mut buf = vec![21u8];
			buf.extend((loopbacks.len() as u32).to_be_bytes());
			// Need ordering, don't use par_iter
			loopbacks.iter().for_each(|name| {
				let name_bytes = name.as_bytes();
				buf.extend((name_bytes.len() as u32).to_be_bytes());
				buf.extend_from_slice(name_bytes);
			});
			buf
		},
		SetSinkVolume(volume) => {
			let mut buf = vec![22u8];
			buf.extend(volume.to_be_bytes());
			buf
		},
		SetFile(path, file) => {
			let mut buf = vec![23u8];
			let path_bytes = path.as_bytes();
			buf.extend((path_bytes.len() as u32).to_be_bytes());
			buf.extend_from_slice(path.as_bytes());
			let mut serialized = vec![];
			file.serialize(&mut rmp_serde::Serializer::new(&mut serialized)).unwrap();
			buf.extend(serialized);
			buf
		},
		SetWave(uid, wave) => {
			let mut buf = vec![24u8];
			buf.extend(uid.to_be_bytes());
			let mut serialized = vec![];
			wave.serialize(&mut rmp_serde::Serializer::new(&mut serialized)).unwrap();
			buf.extend(serialized);
			buf
		},
		SetDialog(uid, dialog) => {
			let mut buf = vec![25u8];
			buf.extend(uid.to_be_bytes());
			let mut serialized = vec![];
			dialog.serialize(&mut rmp_serde::Serializer::new(&mut serialized)).unwrap();
			buf.extend(serialized);
			buf
		},
		DeleteWave(uid) => {
			let mut buf = vec![26u8];
			buf.extend(uid.to_be_bytes());
			buf
		},
		DeleteDialog(uid) => {
			let mut buf = vec![27u8];
			buf.extend(uid.to_be_bytes());
			buf
		},
		SetStopKey(keys) => {
			let mut buf = vec![28u8];
			buf.extend((keys.len() as u32).to_be_bytes());
			// Need ordering, don't use par_iter
			keys.iter().for_each(|key| {
				let key_bytes = key.as_bytes();
				buf.extend((key_bytes.len() as u32).to_be_bytes());
				buf.extend_from_slice(key_bytes);
			});
			buf
		},
		SetPlaylistMode(enabled) => {
			vec![29u8, if *enabled { 1 } else { 0 }]
		},
	}
}

pub fn encode_s2c(response: &ServerToClient) -> Vec<u8> {
	use ServerToClient::*;
	match response {
		Success => vec![1u8],
		Error(message) => {
			let mut buf = vec![2u8];
			buf.extend_from_slice(message.as_bytes());
			buf
		},
		Reload => vec![3u8],
		Playing(playing_type, id, message) => {
			let mut buf = vec![11u8, *playing_type];
			buf.extend(id.to_be_bytes());
			buf.extend_from_slice(message.as_bytes());
			buf
		},
		Stopping(id) => {
			let mut buf = vec![12u8];
			buf.extend(id.to_be_bytes());
			buf
		}
	}
}

pub trait ReadToPause {
	fn read_to_pause(&mut self, pending: &mut Vec<u8>) -> io::Result<bool>;
}

impl ReadToPause for TcpStream {
	fn read_to_pause(&mut self, data: &mut Vec<u8>) -> io::Result<bool> {
		let mut buf = [0; 1024];
		let mut closed = false;
		loop {
			match self.read(&mut buf) {
				Ok(0) => {
					closed = true;
					break;
				},
				Ok(read) => {
					if read >= buf.len() {
						data.extend(buf);
					} else {
						data.extend_from_slice(&buf[..read]);
						break;
					}
				},
				Err(err) => {
					if err.kind() == ErrorKind::TimedOut || err.kind() == ErrorKind::WouldBlock {
						break;
					} else {
						return Err(err);
					}
				}
			}
		}
		Ok(closed)
	}
}