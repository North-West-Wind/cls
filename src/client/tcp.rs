use std::{io::{self, Write}, net::TcpStream, sync::{Arc, atomic::{AtomicBool, Ordering}, mpsc::{self, Receiver, Sender}}, thread, time::Duration};

use parking_lot::Mutex;

use crate::common::{log, socket::{ClientToServer, ReadToPause, ServerToClient, decode_s2c, encode_c2s}};

pub struct TcpClient {
	running: Arc<AtomicBool>,
	request_tx: Sender<Option<ClientToServer>>
}

impl TcpClient {
	pub fn connect(address: &str) -> (Self, Receiver<ServerToClient>) {
		let socket: Arc<Mutex<Option<TcpStream>>> = Arc::new(Mutex::new(None));
		let running = Arc::new(AtomicBool::new(true));

		// Reconnect loop
		{
			let (address, socket, running) = (address.to_string(), socket.clone(), running.clone());
			thread::spawn(move || {
				while running.load(Ordering::Relaxed) {
					let reconnect = socket.lock().is_none();
					if reconnect {
						log::info(format!("TCP client connecting to {}...", address));
						if let Ok(stream) = TcpStream::connect(&address) {
							log::info(format!("TCP client connected to {}", address));
							let _ = stream.set_read_timeout(Some(Duration::from_secs(1)));
							let _ = stream.set_write_timeout(Some(Duration::from_secs(3)));
							let mut socket = socket.lock();
							*socket = Some(stream);
						} else {
							log::info("TCP client connection failed. Retry in 5s...");
							thread::sleep(Duration::from_secs(5));
						}
					} else {
						thread::sleep(Duration::from_secs(1));
					}
				}
			});
		}

		// Subscriber loop
		let (broadcast_tx, broadcast_rx) = mpsc::channel();
		{
			let (socket, running) = (socket.clone(), running.clone());
			thread::spawn(move || -> io::Result<()> {
				let mut buf = vec![];
				while running.load(Ordering::Relaxed) {
					buf.clear();
					let stream = socket.lock().as_ref().map(|s| s.try_clone().unwrap());
					if let Some(mut stream) = stream {
						match stream.read_to_pause(&mut buf) {
							Ok(true) => *socket.lock() = None,
							Ok(false) => {
								if buf.is_empty() {
									continue;
								}
								match decode_s2c(&buf) {
									Ok(msg) => {
										log::info(format!("TCP client decoded s2c: {:?}", msg));
										if let Err(err) = broadcast_tx.send(msg) {
											log::error(format!("TCP client tx send error: {:?}", err));
										}
									},
									Err(err) => log::error(format!("TCP client decode error: {:?}", err)),
								}
							},
							Err(err) => log::error(format!("TCP client read error: {:?}", err)),
						}
					}
				}
				Ok(())
			});
		}

		// Requester loop
		let (request_tx, request_rx) = mpsc::channel();
		{
			let (socket, running) = (socket.clone(), running.clone());
			thread::spawn(move || -> io::Result<()> {
				while running.load(Ordering::Relaxed) {
					match request_rx.recv() {
						Ok(None) => continue,
						Ok(Some(msg)) => {
							let stream = socket.lock().as_ref().map(|s| s.try_clone().unwrap());
							if let Some(mut stream) = stream {
								log::info(format!("TCP client sending c2s: {:?}", msg));
								if let Err(err) = stream.write_all(&encode_c2s(&msg)) {
									log::error(format!("TCP client send error: {:?}", err));
									continue;
								}
							}
						},
						Err(err) => log::error(format!("TCP client recv error: {:?}", err))
					}
				}
				Ok(())
			});
		}

		(Self {
			running,
			request_tx,
		}, broadcast_rx)
	}

	pub fn send(&self, request: ClientToServer) -> Result<(), mpsc::SendError<Option<ClientToServer>>> {
		self.request_tx.send(Some(request))
	}

	pub fn shutdown(&self) {
		self.running.store(false, Ordering::Relaxed);
		let _ = self.request_tx.send(None);
	}
}