use std::{collections::HashMap, io, sync::{Arc, Condvar, Mutex}, thread::{self, JoinHandle}};
use component::block::{files::FilesBlock, help::HelpBlock, playing::PlayingBlock, tabs::TabsBlock, volume::VolumeBlock, BlockComponent};
use constant::{MIN_HEIGHT, MIN_WIDTH};
use util::pulseaudio::{load_null_sink, load_sink_controller, set_volume_percentage, unload_null_sink};
use listener::{listen_events, listen_global_input};
use ratatui::{
    backend::CrosstermBackend,
    Terminal
};
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use state::{Scanning, SharedCondvar};
use util::threads::spawn_scan_thread;
mod renderer;
mod listener;
mod state;
mod config;
mod constant;
mod util;
mod component;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let app = state::get_mut_app();
    app.pair = Option::Some(Arc::new((Mutex::new(SharedCondvar::default()), Condvar::new())));

    app.blocks = vec![
		BlockComponent::Volume(VolumeBlock::default()),
		BlockComponent::Tabs(TabsBlock::default()),
		BlockComponent::Files(FilesBlock::default()),
		BlockComponent::Help(HelpBlock::default()),
        BlockComponent::Playing(PlayingBlock::default()),
    ];
    app.playing = Option::Some(HashMap::new());

    let _ = config::load();
    app.sink_controller = Option::Some(load_sink_controller()?);
    app.module_num = load_null_sink()?;
    set_volume_percentage(app.config.volume);

    app.running = true;

    spawn_scan_thread(Scanning::All);
    let draw_thread = spawn_drawing_thread();
    let listen_thread = spawn_listening_thread();
    listen_thread.join().unwrap()?;
    draw_thread.join().unwrap()?;

    unload_null_sink()?;
    config::save()?;
    Ok(())
}

fn spawn_drawing_thread() -> JoinHandle<Result<(), io::Error>> {
    return thread::spawn(move || -> Result<(), io::Error> {
        // setup terminal
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        let size = terminal.size().unwrap();
        if size.width < MIN_WIDTH || size.height < MIN_HEIGHT {
            let width = size.width;
            let height = size.height;
            let app = state::get_mut_app();
            app.error = String::from(format!("Terminal size requires at least {MIN_WIDTH}x{MIN_HEIGHT}.\nCurrent size: {width}x{height}"));
        }

        let app = state::get_app();
        let pair = app.pair.clone().unwrap();
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

fn spawn_listening_thread() -> JoinHandle<Result<(), io::Error>> {
    return thread::spawn(move || -> Result<(), io::Error> {
        listen_global_input();
        listen_events()?;
        Ok(())
    });
}