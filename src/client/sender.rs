use std::{io::{self, ErrorKind::ConnectionRefused, Write}, net::TcpStream, sync::{Arc, atomic::{AtomicBool, Ordering}, mpsc::{self, Sender}}, thread, time::Duration};

use crate::common::{log, socket::{ClientToServer, ReadToPause, ServerToClient, decode_s2c, encode_c2s}};

pub struct TcpSender {
	running: Arc<AtomicBool>,
	tx: Sender<Option<ClientToServer>>
}

impl TcpSender {
	pub fn connect(address: &str) -> Self {
		let running = Arc::new(AtomicBool::new(true));
		let (tx, rx) = mpsc::channel();

		{
			let address = address.to_string();
			let running = running.clone();
			thread::spawn(move || -> io::Result<()> {
				let mut stream = TcpSender::try_connect(&address, &running)?;
				log::info(format!("TCP sender connected to {}", address));
				let mut buf = vec![];
				while running.load(Ordering::Relaxed) {
					match rx.recv() {
						Ok(None) => continue,
						Ok(Some(msg)) => {
							log::info(format!("TCP sender sending c2s: {:?}", msg));
							if let Err(err) = stream.write_all(&encode_c2s(&msg)) {
								log::error(format!("TCP sender send error: {:?}", err));
								continue;
							}
							buf.clear();
							match stream.read_to_pause(&mut buf) {
								Err(err) => {
									log::error(format!("TCP sender read error: {:?}", err));
									continue;
								},
								Ok(true) => {
									// Server closed. Try to reconnect
									log::info("TCP sender disconnected. Reconnecting...");
									stream = TcpSender::try_connect(&address, &running)?;
									log::info(format!("TCP sender reconnected to {}", address));
								},
								Ok(false) => {
									match decode_s2c(&buf) {
										Err(err) => log::error(err),
										Ok(msg) => {
											log::info(format!("TCP sender received s2c: {:?}", msg));
											if let ServerToClient::Error(message) = msg {
												log::error(message);
											}
										}
									}
								}
							};
						},
						Err(err) => log::error(format!("TCP sender recv error: {:?}", err))
					}
				}
				Ok(())
			});
		}

		Self {
			running,
			tx
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

	pub fn send(&self, request: ClientToServer) -> Result<(), mpsc::SendError<Option<ClientToServer>>> {
		self.tx.send(Some(request))
	}

	pub fn shutdown(&self) {
		self.running.store(false, Ordering::Relaxed);
		self.tx.send(None).unwrap();
	}
}