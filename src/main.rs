use std::{io, sync::{Arc, Condvar, Mutex}, thread::{self, JoinHandle}, time::Duration};
use listener::listen_events;
use ratatui::{
    backend::CrosstermBackend,
    widgets::{Widget, Block, Borders},
    layout::{Layout, Constraint, Direction},
    Terminal
};
use crossterm::{
    event::{poll, read, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use state::{get_running, init_shared_condvar, set_error, set_running, CondvarPair, SharedCondvar};
mod renderer;
mod listener;
mod state;

fn main() -> Result<(), io::Error> {
    state::set_running(true);

    let pair = Arc::new((Mutex::new(init_shared_condvar()), Condvar::new()));
    let pair2 = Arc::clone(&pair);

    let draw_thread = spawn_drawing_thread(pair);
    let listen_thread = spawn_listening_thread(pair2);
    listen_thread.join().unwrap()?;
    draw_thread.join().unwrap()?;

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
            set_error(String::from(format!("Terminal size requires at least 48x11.\nCurrent size: {width}x{height}")));
        }
        while get_running() {
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