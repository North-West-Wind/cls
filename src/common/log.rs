use std::{sync::{Arc, LazyLock, Mutex}, vec};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
	Info,
	Warn,
	Error
}

type Message = (LogLevel, String);
type Callback = Box<dyn Fn(LogLevel, String) + Send + Sync>;

struct Logger {
	messages: Vec<Message>,
	callbacks: Vec<Arc<Callback>>
}

static LOGGER: LazyLock<Mutex<Logger>> = LazyLock::new(|| Mutex::new(Logger { messages: vec![], callbacks: vec![] }));

fn log(level: LogLevel, message: String) {
	let mut logger = LOGGER.lock().unwrap();
	logger.messages.push((level, message.clone()));
	logger.callbacks.iter().for_each(|callback| {
		callback(level, message.clone());
	});
}

pub fn info(message: impl ToString) {
	log(LogLevel::Info, message.to_string());
}

pub fn warn(message: impl ToString) {
	log(LogLevel::Warn, message.to_string());
}

pub fn error(message: impl ToString) {
	log(LogLevel::Error, message.to_string());
}

pub fn register(callback: impl Fn(LogLevel, String) + Send + Sync + 'static) {
	let mut logger = LOGGER.lock().unwrap();
	logger.callbacks.push(Arc::new(Box::new(callback)));
}