use std::{io::{BufWriter, PipeWriter, Write}, process::{Child, ChildStdin}, time::SystemTime};

use cpal::Stream;

pub struct AudioOutput {
	pub last_used: SystemTime,
	pacat: Option<(Child, BufWriter<ChildStdin>)>,
	stream: Option<(Stream, BufWriter<PipeWriter>)>,
}

impl AudioOutput {
	fn get_writer() -> BufWriter<dyn Write> {
		
	}
}

impl Write for AudioOutput {
	fn by_ref(&mut self) -> &mut Self
			where
					Self: Sized, {
			
	}
}