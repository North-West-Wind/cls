use std::{io, sync::{Arc, Condvar, Mutex}, thread::{self, JoinHandle}};
use handler::pulseaudio::{load_null_sink, unload_null_sink};
use listener::listen_events;
use ratatui::{
    backend::CrosstermBackend,
    Terminal
};
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use state::{get_mut_app, CondvarPair, Scanning, SharedCondvar};
use tui_input::Input;
mod renderer;
mod listener;
mod state;
mod config;
mod constant;
mod handler;
mod util;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _ = config::load();

    let app = state::get_mut_app();
    app.module_num = load_null_sink()?;
    app.input = Option::Some(Input::default());

    app.running = true;

    let pair = Arc::new((Mutex::new(SharedCondvar::default()), Condvar::new()));
    let pair2 = Arc::clone(&pair);
    let pair3 = Arc::clone(&pair);

    spawn_scan_thread(pair3, Scanning::ALL);
    let draw_thread = spawn_drawing_thread(pair);
    let listen_thread = spawn_listening_thread(pair2);
    listen_thread.join().unwrap()?;
    draw_thread.join().unwrap()?;

    unload_null_sink()?;
    config::save()?;
    Ok(())
}

fn spawn_drawing_thread(pair: CondvarPair) -> JoinHandle<Result<(), io::Error>> {
    return thread::spawn(move || -> Result<(), io::Error> {
        // setup terminal
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        let size = terminal.size().unwrap();
        if size.width < 48 || size.height < 11 {
            let width = size.width;
            let height = size.height;
            let app = state::get_mut_app();
            app.error = String::from(format!("Terminal size requires at least 48x11.\nCurrent size: {width}x{height}"));
        }

        let app = state::get_app();
        while app.running {
            let (lock, cvar) = &*pair;
            let mut shared = lock.lock().unwrap();
            while !(*shared).redraw {
                shared = cvar.wait(shared).unwrap();
            }
            (*shared).redraw = false;
            terminal.draw(|f| {
                /*let size = f.size();
                let block = Block::default()
                    .title("Block")
                    .borders(Borders::ALL);
                f.render_widget(block, size);*/
                renderer::ui(f);
            })?;
        }

        // restore terminal
        disable_raw_mode()?;
        execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
        terminal.show_cursor()?;
        Ok(())
    });
}

fn spawn_listening_thread(pair: CondvarPair) -> JoinHandle<Result<(), io::Error>> {
    return thread::spawn(move || -> Result<(), io::Error> {
        listen_events(pair)?;
        Ok(())
    });
}

fn spawn_scan_thread(pair: CondvarPair, mode: Scanning) {
    if mode == Scanning::NONE {
        return;
    }
    thread::spawn(move || {
        let app = get_mut_app();
        app.scanning = mode;
        let _ = match mode {
            Scanning::ALL => util::scan_tabs(),
            Scanning::ONE(index) => util::scan_tab(index),
            _ => Ok(())
        };
        app.scanning = Scanning::NONE;
        let (lock, cvar) = &*pair;
        let mut shared = lock.lock().unwrap();
        (*shared).redraw = true;
        cvar.notify_all();
    });
}