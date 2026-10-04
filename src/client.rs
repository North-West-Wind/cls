use std::{collections::HashMap, format, io, sync::Arc, thread::{self, JoinHandle}, time::Duration, vec};

use crossterm::{event::{DisableMouseCapture, EnableMouseCapture}, execute, terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode}};
use indexmap::IndexMap;
use nng::{Error::ConnectionRefused, Protocol, Socket, options::{Options, RecvTimeout, protocol::pubsub::Subscribe}};
use parking_lot::{Condvar, Mutex, RwLock};
use ratatui::{Frame, Terminal, backend::CrosstermBackend, layout::{Alignment, Constraint, Direction, Layout, Rect}, style::{Color, Style}, widgets::{Block, BorderType, Borders, Paragraph}};

use crate::{client::{client_ext::{file::ClientFile, wave::ClientWave}, component::{block::{BlockNavigation, BlockRender, BlockRenderArea, dialogs::DialogBlock, files::FilesBlock, help::HelpBlock, info::InfoBlock, log::LogBlock, playing::PlayingBlock, results::ResultsBlock, search::SearchBlock, settings::SettingsBlock, tabs::TabsBlock, waves::WavesBlock}, popup::{PopupComponent, PopupRender}}, listener::init_key_listener, tab::scan}, common::{base::{dialog::Dialog, wave::Wave}, config::{self, SoundboardConfig}, constant::{ADDRESS_COMMS, ADDRESS_EVENT, MIN_HEIGHT, MIN_WIDTH}, log, socket::{ClientToServer, ServerToClient, decode_s2c, encode_c2s}}};

mod client_ext;
mod component;
mod listener;
mod tab;

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum SelectionLayer {
	Block,
	Content
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum Scanning {
	None,
	All,
	One(usize)
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum MainOpened {
	File,
	Wave,
	Dialog,
	Log,
	Search
}

impl MainOpened {
	pub fn id(&self, fallback: u8) -> u8 {
		match self {
			MainOpened::File => FilesBlock::ID,
			MainOpened::Wave => WavesBlock::ID,
			MainOpened::Dialog => DialogBlock::ID,
			MainOpened::Search => ResultsBlock::ID,
			_ => fallback
		}
	}
}

pub(self) struct StaticBlocks {
	dialogs: DialogBlock,
	files: FilesBlock,
	help: HelpBlock,
	info: InfoBlock,
	log: LogBlock,
	playing: PlayingBlock,
	results: ResultsBlock,
	search: SearchBlock,
	settings: SettingsBlock,
	tabs: TabsBlock,
	waves: WavesBlock
}

pub(self) enum SearchState {
	Initial,
	Searching,
	Finish
}

#[derive(PartialEq, Clone)]
pub(self) struct FileResult {
	parent: String,
	name: String,
	info: ClientFile,
}

#[derive(PartialEq, Clone)]
pub(self) struct SimpleResult {
	uid: u64,
	has_id: bool,
	has_keys: bool,
	main: String,
	sub: String,
}

#[derive(PartialEq, Clone)]
pub(self) enum SearchResult {
	File(FileResult),
	Wave(SimpleResult),
	Dialog(SimpleResult),
}

#[derive(Debug, Clone, Default)]
pub(self) struct Redrawer {
	redraw: Arc<(Mutex<bool>, Condvar)>,
}

impl Redrawer {
	fn notify(&self) {
		let redraw = self.redraw.clone();
		thread::spawn(move || {
			let (lock, cvar) = &*redraw;
			let mut shared = lock.lock();
			*shared = true;
			cvar.notify_one();
		});
	}

	fn wait(&self) {
		let &(ref lock, ref cvar) = &*self.redraw;
		let mut shared = lock.lock();
		if !(*shared) {
			cvar.wait(&mut shared);
		}
		*shared = false;
	}
}

#[derive(Clone)]
pub(self) struct PopupManager {
	popups: Arc<Mutex<Vec<PopupComponent>>>,
	redrawer: Redrawer
}

impl From<Redrawer> for PopupManager {
	fn from(redrawer: Redrawer) -> Self {
		Self {
			popups: Arc::new(Mutex::new(vec![])),
			redrawer
		}
	}
}

impl PopupManager {
	fn push(&self, popup: PopupComponent) -> JoinHandle<()> {
		let popups = self.popups.clone();
		let redrawer = self.redrawer.clone();
		thread::spawn(move || {
			popups.lock().push(popup);
			redrawer.notify();
		})
	}

	fn pop(&self) {
		self.popups.lock().pop();
		self.redrawer.notify();
	}

	fn pop_defer(&self) {
		let popups = self.popups.clone();
		let redrawer = self.redrawer.clone();
		thread::spawn(move || {
			popups.lock().pop();
			redrawer.notify();
		});
	}
}

pub(self) struct ClientState {
	config: SoundboardConfig,
	running: bool,

	// Communication
	socket_comms: Arc<Mutex<Socket>>,

	error: String,
	error_important: bool,
	redrawer: Redrawer,
	selection_layer: SelectionLayer,
	popup_manager: PopupManager,
	settings_opened: bool,
	main_opened: MainOpened,
	scanning: Scanning,
	playing: HashMap<u16, (String, Color)>,
	dirty: bool,

	// Transformed from config
	file_tabs: Vec<(String, IndexMap<String, ClientFile>)>, // (tab path, (file name, file info)[])
	dialogs: Vec<Dialog>,
	waves: Vec<ClientWave>,

	// States for blocks
	selected_block: u8,
	selected_dialog: usize,
	selected_file: usize,
	selected_result: usize,
	selected_tab: usize,
	selected_wave: usize,

	search_results: Vec<(i64, SearchResult)>,
	search_state: SearchState,
}

pub(self) type AtomicStaticBlocks = Arc<RwLock<StaticBlocks>>;
pub(self) type AtomicClientState = Arc<RwLock<ClientState>>;

impl ClientState {
	fn load_config(&mut self) {
		self.config = config::load();
	}

	fn apply_config(&mut self) {
		let config = &self.config;
		self.dialogs.clear();

		self.file_tabs = config.tabs.iter().map(|tab| {
			let mut files = IndexMap::new();
			if let Some(config_files) = config.files.get(tab) {
				config_files.iter().for_each(|(name, file)| {
					files.insert(name.clone(), ClientFile::from(file.clone()));
				});
			}
			(tab.clone(), files)
		}).collect();
		// REMEMBER TO SCAN TABS AFTER THIS

		self.waves = config.waves.iter().map(|wave| ClientWave::from(Wave::from(wave))).collect();
		self.dialogs = config.dialogs.iter().map(|dialog| Dialog::from(dialog)).collect();
	}

	fn save_config(&mut self) {
		self.config.files.clear();
		self.file_tabs.iter().for_each(|(tab, files)| {
			let mut config_files = HashMap::new();
			files.iter().for_each(|(name, file)| {
				config_files.insert(name.clone(), file.base.clone());
			});
			self.config.files.insert(tab.clone(), config_files);
		});

		self.config.waves = self.waves.iter().map(|wave| wave.base.clone().into()).collect();
		self.config.dialogs = self.dialogs.iter().map(|dialog| dialog.clone().into()).collect();

		config::save(&self.config);

		// Reload server
		self.request(ClientToServer::Reload);
	}

	fn request(&self, request: ClientToServer) -> bool {
		if let Err((_, err)) = self.socket_comms.lock().send(&encode_c2s(request)) {
			log::error(err);
			return false;
		};
		let socket_comms = self.socket_comms.clone();
		thread::spawn(move || {
			let result = {
				match socket_comms.lock().recv() {
					Ok(mut msg) => decode_s2c(&mut msg),
					Err(err) => {
						log::error(err);
						return;
					}
				}
			};
			match result {
				Err(err) => log::error(err),
				Ok(ServerToClient::Error(message)) => log::error(message),
				_ => ()
			}
		});
		true
	}

	fn borders(&self, id: u8) -> (BorderType, Style) {
		let style = Style::default().fg(
			if self.selected_block == id {
				Color::White
			} else {
				Color::DarkGray
			}
		);
		let border_type = if self.selected_block == id {
			if self.selection_layer == SelectionLayer::Content {
				BorderType::Double
			} else {
				BorderType::Thick
			}
		} else {
			BorderType::Rounded
		};
		(border_type, style)
	}

	fn get_file(&self) -> Option<(String, String, ClientFile)> {
		if self.selected_tab >= self.file_tabs.len() {
			return None;
		}
		let (tab, files) = &self.file_tabs[self.selected_tab];
		if self.selected_file >= files.len() {
			return None;
		}
		let mut files = files.iter().map(|(name, info)| (name, info)).collect::<Vec<_>>();
		files.sort_by(|(a, _), (b, _)| a.cmp(b));
		let (name, info) = files[self.selected_file];
		return Some((tab.clone(), name.clone(), info.clone()));
	}

	fn exit(&mut self) {
		self.running = false;
		self.redrawer.notify();
	}
}

pub fn start_client(save_on_exit: bool) -> Result<(), Box<dyn std::error::Error>> {
	let mut retries = 5;
	let socket_comms = Socket::new(Protocol::Req0)?;
	while let Err(ConnectionRefused) = socket_comms.dial(ADDRESS_COMMS) {
		retries -= 1;
		if retries == 0 {
			return Err(Box::new(ConnectionRefused));
		} else {
			thread::sleep(Duration::from_secs(1));
		}
	}
	retries = 5;
	let socket_event = Socket::new(Protocol::Sub0)?;
	while let Err(ConnectionRefused) = socket_event.dial(ADDRESS_EVENT) {
		retries -= 1;
		if retries == 0 {
			return Err(Box::new(ConnectionRefused));
		} else {
			thread::sleep(Duration::from_secs(1));
		}
	}

	socket_comms.set_opt::<RecvTimeout>(Some(Duration::from_secs(3)))?;
	socket_event.set_opt::<RecvTimeout>(Some(Duration::from_secs(3)))?;
	socket_event.set_opt::<Subscribe>(vec![])?; // Subscribe to all topics

	let redrawer = Redrawer::default();

	let client_state = Arc::new(RwLock::new(ClientState {
		config: config::load(),
		running: true,

		socket_comms: Arc::new(Mutex::new(socket_comms)),

		error: String::new(),
		error_important: false,
		popup_manager: PopupManager::from(redrawer.clone()),
		redrawer: redrawer.clone(),
		selection_layer: SelectionLayer::Block,
		settings_opened: false,
		main_opened: MainOpened::File,
		scanning: Scanning::None,
		file_tabs: vec![],
		dialogs: vec![],
		waves: vec![],
		playing: HashMap::new(),
		dirty: false,

		selected_block: 0,
		selected_dialog: 0,
		selected_file: 0,
		selected_result: 0,
		selected_tab: 0,
		selected_wave: 0,

		search_results: vec![],
		search_state: SearchState::Initial
	}));

	client_state.write().apply_config();
	{
		let client_state = client_state.clone();
		thread::spawn(move || scan(client_state, Scanning::All));
	}

	let blocks = Arc::new(RwLock::new(StaticBlocks {
		dialogs: DialogBlock::default(),
		files: FilesBlock::default(),
		help: HelpBlock::default(),
		info: InfoBlock::default(),
		log: LogBlock::new(redrawer),
		playing: PlayingBlock::default(),
		results: ResultsBlock::default(),
		search: SearchBlock::default(),
		settings: SettingsBlock::default(),
		tabs: TabsBlock::default(),
		waves: WavesBlock::default(),
	}));

	// Event socket listener
	let client_state_socket = client_state.clone();
	thread::spawn(move || {
		while client_state_socket.read().running {
			match socket_event.recv() {
				Ok(mut msg) => {
					log::info(format!("Received server broadcast: {:?}", msg));
					match decode_s2c(&mut msg) {
						Ok(s2c) => {
							use ServerToClient::*;
							match s2c {
								Error(err) => log::error(err),
								Reload => {
									let mut client_state = client_state_socket.write();
									client_state.load_config();
									client_state.apply_config();
									scan(client_state_socket.clone(), Scanning::All);
								},
								Playing(playing_type, id, message) => {
									let mut client_state = client_state_socket.write();
									let color = match playing_type {
										1 => Color::LightCyan,
										2 => Color::LightYellow,
										_ => Color::LightGreen
									};
									client_state.playing.insert(id, (message, color));
									client_state.redrawer.notify();
								},
								Stopping(id) => {
									let mut client_state = client_state_socket.write();
									client_state.playing.remove(&id);
									client_state.redrawer.notify();
								},
								_ => ()
							}
						},
						Err(err) => log::error(format!("Failed to decode server broadcast: {:?}", err)),
					}
				},
				Err(err) if err == nng::Error::TimedOut => (), // Ignore server timeout
				Err(err) => log::error(format!("Failed to recv server broadcast: {:?}", err)),
			}
		}
	});

	// Key listeners
	let client_state_key = client_state.clone();
	let blocks_key = blocks.clone();
	thread::spawn(move || { let _ = init_key_listener(client_state_key, blocks_key); });

	// Termination signal handler
	let client_state_signal = client_state.clone();
	let _ = ctrlc::set_handler(move || client_state_signal.write().exit());

	// Setup terminal
	enable_raw_mode()?;
	let mut stdout = io::stdout();
	execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
	let backend = CrosstermBackend::new(stdout);
	let mut terminal = Terminal::new(backend)?;

	// Check minimum terminal size
	let size = terminal.size()?;
	if size.width < MIN_WIDTH || size.height < MIN_HEIGHT {
		let width = size.width;
		let height = size.height;
		let mut client_state = client_state.write();
		client_state.error = String::from(format!("Terminal size requires at least {MIN_WIDTH}x{MIN_HEIGHT}.\nCurrent size: {width}x{height}"));
		client_state.error_important = true;
	}

	// Render to the terminal
	let (redrawer, popup_manager) = {
		let client_state = client_state.read();
		(client_state.redrawer.clone(), client_state.popup_manager.clone())
	};
	while client_state.read().running {
		// Render again
		if let Err(err) = terminal.draw(|f| {
			draw_blocks(&client_state.read(), &mut blocks.write(), f);
			draw_popups(&popup_manager.popups.lock(), f);
		}) {
			log::error(err);
			break;
		}
		redrawer.wait();
	}

	// Restore terminal
	disable_raw_mode()?;
	execute!(
		terminal.backend_mut(),
		LeaveAlternateScreen,
		DisableMouseCapture
	)?;
	terminal.show_cursor()?;

	if save_on_exit {
		client_state.write().save_config();
	}

	blocks.write().log.flush_logs();

	log::info("Client is done. Goodbye!");

	Ok(())
}

fn draw_blocks(client_state: &ClientState, blocks: &mut StaticBlocks, f: &mut Frame) {
	let (error, settings, main_opened) = (client_state.error.clone(), client_state.settings_opened, client_state.main_opened);

	if !error.is_empty() {
		return draw_error(error, f);
	}

 	let chunks = Layout::default()
		.direction(Direction::Vertical)
		.margin(1)
		.constraints(
			[
				Constraint::Length(7),
				Constraint::Length(3),
				Constraint::Fill(1),
				Constraint::Length(1)
			].as_ref()
		)
		.split(f.area());

	if main_opened == MainOpened::Log {
		blocks.log.render_area(client_state, f, f.area());
		return;
	}
	blocks.info.render_area(client_state, f, chunks[0]);
	if main_opened == MainOpened::Search {
		blocks.search.render_area(client_state, f, chunks[1]);
	} else {
		blocks.tabs.render_area(client_state, f, chunks[1]);
	}
	let files_area: Rect;
	if settings {
		let mid_chunks1 = Layout::default().direction(Direction::Horizontal).constraints([Constraint::Fill(1), Constraint::Length(20)].as_ref()).split(chunks[2]);
		let mid_chunks2 = Layout::default().direction(Direction::Horizontal).constraints([Constraint::Fill(1), Constraint::Percentage(30)].as_ref()).split(chunks[2]);
		let mid_chunks;
		// Settings have minimum 20 char width
		if mid_chunks1[1].width > mid_chunks2[1].width {
			mid_chunks = mid_chunks1;
		} else {
			mid_chunks = mid_chunks2;
		}
		files_area = mid_chunks[0];
		blocks.settings.render_area(client_state, f, mid_chunks[1]);
	} else {
		files_area = chunks[2];
	}
	match main_opened {
		MainOpened::File => blocks.files.render_area(client_state, f, files_area),
		MainOpened::Wave => blocks.waves.render_area(client_state, f, files_area),
		MainOpened::Dialog => blocks.dialogs.render_area(client_state, f, files_area),
		MainOpened::Search => blocks.results.render_area(client_state, f, files_area),
		_ => ()
	}
	blocks.help.render_area(client_state, f, chunks[3]);
	blocks.playing.render(client_state, f);
	// No parallel. Need to draw in order
	let popups = client_state.popup_manager.popups.clone();
	let popups = popups.lock();
	popups.iter().for_each(|popup| popup.render(f));
}

fn draw_popups(popups: &Vec<PopupComponent>, f: &mut Frame) {
	popups.iter().for_each(|popup| popup.render(f));
}

fn draw_error(error: String, f: &mut Frame) {
	let paragraph = Paragraph::new(error)
		.alignment(Alignment::Center)
		.style(Style::default().fg(Color::Red))
		.block(
			Block::default()
				.borders(Borders::ALL)
				.border_type(BorderType::Rounded)
		);
	f.render_widget(paragraph, f.area());
}