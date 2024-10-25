use std::{collections::HashMap, io, sync::{Arc, Condvar, Mutex}, thread::{self, JoinHandle}, time::Duration};
use component::block::{files::FilesBlock, help::HelpBlock, playing::PlayingBlock, settings::SettingsBlock, tabs::TabsBlock, volume::VolumeBlock, BlockComponent};
use constant::{MIN_HEIGHT, MIN_WIDTH};
use signal_hook::iterator::Signals;
use socket::{ensure_socket, listen_socket, send_exit, send_socket};
use std_semaphore::Semaphore;
use util::pulseaudio::{load_null_sink, load_sink_controller, loopback, set_volume_percentage, unload_modules};
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
use clap::{command, Arg, ArgAction, Command};
mod component;
mod config;
mod constant;
mod listener;
mod renderer;
mod socket;
mod state;
mod util;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut command = command!()
        .about("Command-Line Soundboard")
        .disable_help_flag(true)
        .disable_help_subcommand(true)
        .args_conflicts_with_subcommands(true)
        .arg(Arg::new("help").short('h').long("help").help("print this help menu").action(ArgAction::SetTrue))
        .arg(Arg::new("edit").short('e').long("edit").help("run the soundboard in edit mode, meaning you can only modify config and not play anything").action(ArgAction::SetTrue))
        .arg(Arg::new("hidden").long("hidden").help("run the soundboard in the background, basically read-only").action(ArgAction::SetTrue))
        .subcommand(Command::new("exit").about("exit another instance"))
        .subcommand(Command::new("reload-config").about("reload config for another instance"))
        .subcommand(Command::new("add-tab").about("add a directory tab").arg(Arg::new("dir").required(true)))
        .subcommand(Command::new("delete-tab").about("delete a tab, defaults to the selected one")
            .args([
                Arg::new("index").long("index").help("delete a specific index (starting at 0)"),
                Arg::new("path").long("path").help("delete the tab with this path"),
                Arg::new("name").long("name").help("delete the tab with this basename")
            ]))
        .subcommand(Command::new("reload-tab").about("reload a tab, defaults to the selected one").args([
            Arg::new("index").long("index").help("reload a specific index (starting at 0)"),
            Arg::new("path").long("path").help("reload the tab with this path"),
            Arg::new("name").long("name").help("reload the tab with this basename")
        ]))
        .subcommand(Command::new("play").about("play a file").arg(Arg::new("path").required(true)))
        .subcommand(Command::new("stop").about("stop all playing files"))
        .subcommand(Command::new("set-volume").about("set volume of the sink or a file").args([
            Arg::new("volume").help("new volume or volume increment (-200 - +200)"),
            Arg::new("increment").long("increment").help("increment volume instead of setting it").action(ArgAction::SetTrue),
            Arg::new("path").long("path").help("a file's volume to set")
        ]));

    let matches = command.clone().get_matches();
    if matches.get_flag("help") {
        command.print_help()?;
        return Ok(());
    }
    let subcommand = matches.subcommand();
    if subcommand.is_some() {
        send_socket(subcommand.unwrap())?;
        return Ok(());
    }
    let app = state::get_mut_app();
    app.hidden = matches.get_flag("hidden");
    app.edit = matches.get_flag("edit");

    if app.hidden && app.edit {
        println!("`hidden` is read-only, but `edit` is write-only.");
        println!("You probably don't want this");
        return Ok(());
    }

    app.pair = Option::Some(Arc::new((Mutex::new(SharedCondvar::default()), Condvar::new())));

    // variables setup
    app.blocks = vec![
		BlockComponent::Volume(VolumeBlock::default()),
		BlockComponent::Tabs(TabsBlock::default()),
		BlockComponent::Files(FilesBlock::default()),
        BlockComponent::Settings(SettingsBlock::default()),
		BlockComponent::Help(HelpBlock::default()),
        BlockComponent::Playing(PlayingBlock::default()),
    ];
    app.playing_file = Option::Some(HashMap::new());
    app.playing_process = Option::Some(HashMap::new());
    app.playing_semaphore = Option::Some(Semaphore::new(1));
    if !app.edit {
        ensure_socket();
        if !app.socket_holder {
            println!("Found existing socket! That probably means another instance is running. Forcing edit mode...");
            app.edit = true;
            thread::sleep(Duration::from_secs(3));
        }
    }

    // pulseaudio setup
    let result = config::load();
    if result.is_err() {
        panic!("{:?}", result.err());
    }
    app.sink_controller = Option::Some(load_sink_controller()?);
    if !app.edit {
        app.module_nums.push(load_null_sink()?);
        if !app.config.loopback_1.is_empty() {
            app.module_nums.push(loopback(app.config.loopback_1.clone())?);
        }
        if !app.config.loopback_2.is_empty() {
            app.module_nums.push(loopback(app.config.loopback_2.clone())?);
        }
    }
    set_volume_percentage(app.config.volume);

    app.running = true;

    spawn_signal_thread()?;
    spawn_scan_thread(Scanning::All);
    let listen_thread = spawn_listening_thread();
    let mut socket_thread = Option::None;
    if app.socket_holder {
        socket_thread = Option::Some(spawn_socket_thread());
    }
    if !app.hidden {
        let draw_thread = spawn_drawing_thread();
        draw_thread.join().unwrap()?;
    }
    listen_thread.join().unwrap()?;
    if socket_thread.is_some() {
        send_exit()?;
        socket_thread.unwrap().join().unwrap()?;
    }

    unload_modules()?;
    if !app.hidden {
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
            app.error_important = true;
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

fn spawn_socket_thread() -> JoinHandle<Result<(), io::Error>> {
    return thread::spawn(move || -> Result<(), io::Error> {
        listen_socket()?;
        Ok(())
    });
}