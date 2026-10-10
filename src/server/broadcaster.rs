use std::{io::{self, Write}, net::{TcpListener, TcpStream}, sync::{Arc, atomic::{AtomicBool, Ordering}}, thread, time::Duration};

use parking_lot::Mutex;

use crate::common::{log, socket::{ServerToClient, encode_s2c}};

#[derive(Debug, Clone)]
pub struct TcpBroadcaster {
	address: String,
	running: Arc<AtomicBool>,
	clients: Arc<Mutex<Vec<TcpStream>>>
}

impl TcpBroadcaster {
	pub fn bind(address: &str) -> io::Result<Self> {
		let listener = TcpListener::bind(address)?;
		log::info(format!("TCP broadcaster listening to {}", address));
		let running = Arc::new(AtomicBool::new(true));
		let clients = Arc::new(Mutex::new(vec![]));

		{
			let running = running.clone();
			let clients = clients.clone();
			thread::spawn(move || -> io::Result<()> {
				while running.load(Ordering::Relaxed) {
					match listener.accept() {
						Ok((stream, addr)) => {
							log::info(format!("Subscriber client connected from {}", addr));
							stream.set_read_timeout(Some(Duration::from_secs(1)))?;
							stream.set_write_timeout(Some(Duration::from_secs(3)))?;
							clients.lock().push(stream);
						},
						Err(err) => log::error(format!("Failed to accept client: {:?}", err)),
					}
				}
				Ok(())
			});
		}

		Ok(Self {
			address: address.to_string(),
			running,
			clients
		})
	}

	pub fn send(&self, message: ServerToClient) {
		log::info(format!("TCP broadcaster sending s2c: {:?}", message));
		self.clients.lock().retain_mut(|client| {
			client.write_all(&encode_s2c(&message)).is_ok()
		});
	}

	pub fn shutdown(&self) {
		self.running.store(false, Ordering::Relaxed);
		// Connect to listener to break out of loop
		TcpStream::connect(&self.address).unwrap();
	}
}