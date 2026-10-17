use std::{io::{self, Write}, net::{TcpListener, TcpStream}, sync::{Arc, atomic::{AtomicBool, Ordering}, mpsc}, thread, time::Duration};

use parking_lot::{Condvar, Mutex};

use crate::common::{log, socket::{ClientToServer, ReadToPause, ServerToClient, decode_c2s, encode_s2c}};

pub struct Request {
	msg: ClientToServer,
	responder: Arc<(Mutex<Option<ServerToClient>>, Condvar)>
}

impl Request {
	pub fn msg(&self) -> &ClientToServer {
		&self.msg
	}

	pub fn reply(&self, message: ServerToClient) {
		let (lock, cvar) = &*self.responder;
		let mut response = lock.lock();
		*response = Some(message);
		cvar.notify_one();
	}
}

// Handles client requests and broadcasting
#[derive(Debug, Clone)]
pub struct TcpServer {
	address: String,
	running: Arc<AtomicBool>,
	clients: Arc<Mutex<Vec<TcpStream>>>
}

impl TcpServer {
	pub fn bind(address: &str) -> io::Result<(Self, mpsc::Receiver<Request>)> {
		let listener = TcpListener::bind(address)?;
		log::info(format!("TCP server listening to {}", address));
		let running = Arc::new(AtomicBool::new(true));
		let clients = Arc::new(Mutex::new(vec![]));
		let (tx, rx) = mpsc::channel();

		{
			let running = running.clone();
			let clients = clients.clone();
			thread::spawn(move || -> io::Result<()> {
				while running.load(Ordering::Relaxed) {
					match listener.accept() {
						Ok((mut stream, addr)) => {
							log::info(format!("Receiver client connected from {}", addr));
							stream.set_read_timeout(Some(Duration::from_secs(1)))?;
							stream.set_write_timeout(Some(Duration::from_secs(3)))?;
							clients.lock().push(stream.try_clone().unwrap());
							let running = running.clone();
							let tx = tx.clone();
							thread::spawn(move || {
								let mut buf = vec![];
								while running.load(Ordering::Relaxed) {
									buf.clear();
									match stream.read_to_pause(&mut buf) {
										Ok(true) => break,
										Ok(false) => {
											if buf.is_empty() {
												continue;
											}
											match decode_c2s(&buf) {
												Ok(msg) => {
													log::info(format!("TCP server decoded c2s: {:?}", msg));
													let responder = Arc::new((Mutex::new(None), Condvar::new()));
													let request = Request {
														msg,
														responder: responder.clone(),
													};

													tx.send(request).unwrap();

													let (lock, cvar) = &*responder;
													let mut response = lock.lock();
													while response.is_none() {
														cvar.wait(&mut response);
													}

													let _ = stream.write_all(&encode_s2c(response.as_ref().unwrap()));
												},
												Err(err) => {
													log::error(format!("Message decode error: {:?}", err));
													let _ = stream.write_all(&encode_s2c(&ServerToClient::Error(err.to_string())));
												}
											}
										},
										Err(err) => log::error(format!("TCP receiver read error: {:?}", err))
									}
								}
								log::info(format!("Receiver client {} disconnected", addr));
							});
						},
						Err(err) => log::error(format!("Failed to accept client: {:?}", err)),
					}
				}
				Ok(())
			});
		}

		Ok((Self {
			address: address.to_string(),
			running,
			clients,
		}, rx))
	}

	pub fn send(&self, message: ServerToClient) {
		log::info(format!("TCP server sending s2c: {:?}", message));
		self.clients.lock().retain_mut(|client| {
			client.write_all(&encode_s2c(&message)).is_ok()
		});
	}

	pub fn shutdown(&self) {
		self.running.store(false, Ordering::Relaxed);
		self.clients.lock().clear();
		// Connect to listener to break out of loop
		TcpStream::connect(&self.address).unwrap();
	}
}