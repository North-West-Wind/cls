use std::{fmt, vec, write};

use nng::Message;
use uuid::Uuid;

#[derive(Debug, Clone)]
struct DecodeError;

impl std::error::Error for DecodeError {}

impl fmt::Display for DecodeError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "socket stream decode failed")
	}
}

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
}

pub enum ServerToClient {
	// Starts from 1
	Success,
	Error(String),
	Reload,

	// Starts from 11
	Playing(u8, Uuid, String),
	Stopping(Uuid)
}

pub fn decode_c2s(msg: &Message) -> Result<ClientToServer, Box<dyn std::error::Error>> {
	use ClientToServer::*;
	match msg[0] {
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
		_ => Err(Box::from(DecodeError))
	}
}

pub fn decode_s2c(msg: &Message) -> Result<ServerToClient, Box<dyn std::error::Error>> {
	use ServerToClient::*;
	match msg[0] {
		1 => Ok(Success),
		2 => {
			let message = String::from_utf8(msg[1..].try_into()?)?;
			Ok(Error(message))
		},
		3 => Ok(Reload),
		11 => {
			let uuid = Uuid::from_bytes(msg[2..17].try_into()?);
			let body = String::from_utf8(msg[17..].try_into()?)?;
			Ok(Playing(msg[1], uuid, body))
		},
		12 => {
			let uuid = Uuid::from_bytes(msg[1..].try_into()?);
			Ok(Stopping(uuid))
		},
		_ => Err(Box::from(DecodeError))
	}
}

pub fn encode_c2s(request: ClientToServer) -> Vec<u8> {
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
		}
	}
}

pub fn encode_s2c(response: ServerToClient) -> Vec<u8> {
	use ServerToClient::*;
	match response {
		Success => vec![1u8],
		Error(message) => {
			let mut buf = vec![2u8];
			buf.extend_from_slice(message.as_bytes());
			buf
		},
		Reload => vec![3u8],
		Playing(playing_type, uuid, message) => {
			let mut buf = vec![11u8, playing_type];
			buf.extend_from_slice(uuid.as_bytes());
			buf.extend_from_slice(message.as_bytes());
			buf
		},
		Stopping(uuid) => {
			let mut buf = vec![12u8];
			buf.extend_from_slice(uuid.as_bytes());
			buf
		}
	}
}