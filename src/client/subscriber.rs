use std::{io::{self, ErrorKind::ConnectionRefused}, net::TcpStream, sync::{Arc, atomic::{AtomicBool, Ordering}, mpsc::{self, Receiver, RecvError}}, thread, time::Duration};

use crate::common::{log, socket::{ReadToPause, ServerToClient, decode_s2c}};

pub struct TcpSubscriber {
	running: Arc<AtomicBool>,
	rx: Receiver<ServerToClient>
}

impl TcpSubscriber {
	pub fn connect(address: &str) -> Self {
		let running = Arc::new(AtomicBool::new(true));
		let (tx, rx) = mpsc::channel();

		{
			let address = address.to_string();
			let running = running.clone();
			thread::spawn(move || -> io::Result<()> {
				let mut stream = TcpSubscriber::try_connect(&address, &running)?;
				log::info(format!("TCP subscriber connected to {}", address));
				let mut buf = vec![];
				while running.load(Ordering::Relaxed) {
					buf.clear();
					match stream.read_to_pause(&mut buf) {
						Ok(true) => {
							// Server closed. Reconnect
							log::info("TCP subscriber disconnected. Reconnecting...");
							stream = TcpSubscriber::try_connect(&address, &running)?;
							log::info(format!("TCP subscriber reconnected to {}", address));
						},
						Ok(false) => {
							if buf.is_empty() {
								continue;
							}
							match decode_s2c(&buf) {
								Ok(msg) => {
									log::info(format!("TCP subscriber decoded s2c: {:?}", msg));
									if let Err(err) = tx.send(msg) {
										log::error(format!("TCP subscriber tx send error: {:?}", err));
									}
								},
								Err(err) => log::error(format!("TCP subscriber decode error: {:?}", err)),
							}
						},
						Err(err) => log::error(format!("TCP subscriber read error: {:?}", err)),
					}
				}
				Ok(())
			});
		}

		Self {
			running,
			rx
		}
	}

	fn try_connect(address: &str, running: &AtomicBool) -> io::Result<TcpStream> {
		let mut stream = TcpStream::connect(address);
		while let Err(ref err) = stream && running.load(Ordering::Relaxed) {
			if err.kind() != ConnectionRefused {
				return Err(io::Error::new(err.kind(), err.to_string()));
			}
			thread::sleep(Duration::from_secs(1));
			stream = TcpStream::connect(address);
		}
		let stream = stream?;
		stream.set_read_timeout(Some(Duration::from_secs(1)))?;
		stream.set_write_timeout(Some(Duration::from_secs(3)))?;
		Ok(stream)
	}

	pub fn recv(&self) -> Result<ServerToClient, RecvError> {
		self.rx.recv()
	}

	pub fn shutdown(&self) {
		self.running.store(false, Ordering::Relaxed);
		// TcpStream should time out on its own
	}
}