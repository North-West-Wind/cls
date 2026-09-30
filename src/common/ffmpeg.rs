use std::{format, io::Read, process::{Command, Stdio}};

use crate::common::constant::ENDIANESS;

pub fn read_file_ffmpeg(path: &str, sample_rate: u32) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
	let result = Command::new("ffmpeg").args([
		"-loglevel", "-8",
		"-i", path,
		"-f", format!("f32{}", ENDIANESS).as_str(),
		"-ac", "2",
		"-ar", sample_rate.to_string().as_str(),
		"-"
	]).stdout(Stdio::piped()).spawn()?;
	let mut buf = vec![];
	let _ = result.stdout.unwrap().read_to_end(&mut buf);
	Ok(buf.chunks(4).map(|chunk| f32::from_le_bytes(chunk.try_into().unwrap())).collect())
}