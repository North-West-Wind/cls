use std::{collections::HashMap, io, sync::{Arc, Condvar, Mutex}, thread::{self, JoinHandle}};
use component::block::{files::FilesBlock, help::HelpBlock, playing::PlayingBlock, tabs::TabsBlock, volume::VolumeBlock, BlockComponent};
use constant::{MIN_HEIGHT, MIN_WIDTH};
use external::dbus::start_zbus;
use getopts::Options;
use signal_hook::iterator::Signals;
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
use state::{get_mut_app, Scanning, SharedCondvar};
use util::threads::spawn_scan_thread;
mod component;
mod config;
mod constant;
mod external;
mod listener;
mod renderer;
mod state;
mod util;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let program = args[0].clone();

    let mut opts = Options::new();
    opts.optflag("h", "help", "print this help menu");
    opts.optflag("", "hidden", "run the soundboard in the background");
    opts.optflag("", "dbus", "allow control over dbus");
    let matches = match opts.parse(&args[1..]) {
        Ok(m) => { m },
        Err(f) => { panic!("{}", f.to_string()) }
    };
    if matches.opt_present("h") {
        print_usage(&program, opts);
        return Ok(());
    }
    let hidden = matches.opt_present("hidden");

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

    if matches.opt_present("dbus") {
        start_zbus();
    }
    spawn_signal_thread()?;
    spawn_scan_thread(Scanning::All);
    let listen_thread = spawn_listening_thread(hidden);
    if !hidden {
        let draw_thread = spawn_drawing_thread();
        draw_thread.join().unwrap()?;
    }
    listen_thread.join().unwrap()?;

    unload_null_sink()?;
    if !hidden {
        config::save()?;
    }
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

fn spawn_listening_thread(no_listen: bool) -> JoinHandle<Result<(), io::Error>> {
    return thread::spawn(move || -> Result<(), io::Error> {
        listen_global_input();
        listen_events(no_listen)?;
        Ok(())
    });
}

fn spawn_signal_thread() -> Result<JoinHandle<()>, Box<dyn std::error::Error>> {
    use signal_hook::consts::*;
    let mut signals = Signals::new([SIGINT, SIGTERM])?;
    return Ok(thread::spawn(move || {
        for sig in signals.forever() {
            let app = get_mut_app();
            match sig {
                SIGINT|SIGTERM => {
                    app.running = false;
                    break;
                },
                _ => (),
            }
        }
    }));
}

fn print_usage(program: &str, opts: Options) {
    let brief = format!("Usage: {} [options]", program);
    print!("{}", opts.usage(&brief));
}