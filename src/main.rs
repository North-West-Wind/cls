use std::{io, sync::{Arc, Condvar, Mutex}, thread::{self, JoinHandle}, time::Duration};
use listener::listen_events;
use ratatui::{
    backend::CrosstermBackend,
    widgets::{Widget, Block, Borders},
    layout::{Layout, Constraint, Direction},
    Terminal
};
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use state::set_running;
mod renderer;
mod listener;
mod state;

fn main() -> Result<(), io::Error> {
    state::set_running(true);

    let pair = Arc::new((Mutex::new(true), Condvar::new()));
    let pair2 = Arc::clone(&pair);

    let draw_thread = spawn_drawing_thread(pair);
    let listen_thread = spawn_listening_thread(pair2);
    thread::sleep(Duration::from_millis(5000));
    set_running(false);
    listen_thread.join().unwrap()?;
    draw_thread.join().unwrap()?;

    Ok(())
}

fn spawn_drawing_thread(pair: Arc<(Mutex<bool>, Condvar)>) -> JoinHandle<Result<(), io::Error>> {
    return thread::spawn(move || -> Result<(), io::Error> {
        // setup terminal
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        while unsafe { state::RUNNING } {
            let (lock, cvar) = &*pair;
            let mut redraw = lock.lock().unwrap();
            while !*redraw {
                redraw = cvar.wait(redraw).unwrap();
            }
            *redraw = false;
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

fn spawn_listening_thread(pair: Arc<(Mutex<bool>, Condvar)>) -> JoinHandle<Result<(), io::Error>> {
    return thread::spawn(move || -> Result<(), io::Error> {
        listen_events(pair)?;
        Ok(())
    });
}