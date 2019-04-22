extern crate i2cdev;

#[macro_use]
extern crate log;
extern crate env_logger;

extern crate music;

use std::collections::BTreeSet;
use std::env;
use std::process;
use std::sync::mpsc;

mod input;
mod simulation;

use input::bitevents::BitEvent;

#[derive(Copy, Clone, Hash, PartialEq, Eq)]
enum Music {
    Background,
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() != 4 {
        eprintln!(
            "Usage: {} <i2c dev path 1> <i2c dev path 2> [<event handler file>]",
            args[0]
        );
        process::exit(-1);
    }

    env_logger::init();

    // Set up a channel for simulation feedback
    let (tx, rx) = mpsc::channel::<BitEvent>();

    info!("Init sound...");

    music::start::<Music, &'static String, _>(16, || {
        music::bind_music_file(Music::Background, "./sounds/background.wav");
        music::set_volume(music::MAX_VOLUME);

        info!("Starting music...");
        music::play_music(&Music::Background, music::Repeat::Forever);

        let handlers = load_handlers(&args[3]).expect("Failed to load handlers");

        let sim = init_simulator(&tx, handlers);

        info!("Configuring devices...");

        if args[1].to_lowercase() == "stdin" {
            debug!("Read Stdin");
            main_loop(&mut input::stdin::StdinInput::new(), rx, sim);
        } else {
            debug!("Read MCP23017");
            main_loop(
                &mut input::mcp23017::PanelInputHandler::new(&args)
                    .expect("Could not init MCP23017s"),
                rx,
                sim,
            );
        }

        info!("Sound complete");
    });
}

fn main_loop<T: input::InputHandler>(
    input: &mut T,
    rx: mpsc::Receiver<BitEvent>,
    sim: simulation::Simulator,
) {
    loop {
        // fetch any pending handler feedback events
        let feedback_events: Vec<BitEvent> = rx.try_iter().collect();

        if !feedback_events.is_empty() {
            input.set_output(3, &feedback_events).unwrap_or_else(|err| {
                warn!("Error setting outputs {:?}: {}", feedback_events, err);
            });
        }

        match input.read_events() {
            Ok(ref events) if !events.is_empty() => {
                info!("Read {:?}", events);
                sim.process(&events);
            }
            Ok(_) => {
                // Noop on empty input
            }
            Err(e) => {
                error!("Error reading events: {}", e);
            }
        }
    }
}

// Globals for now, need to encapsulate state later

fn init_simulator(sender: &mpsc::Sender<BitEvent>, handlers: HandlerMap) -> simulation::Simulator {
    use simulation::*;

    Simulator::new(handlers, &sender)
}

use input::InputError;
use simulation::{EventHandler, HandlerMap};
use std::collections::BTreeMap;
use std::fs::File;
use std::io::{BufRead, BufReader};

// Format for each line is "input ID, <name>, <on sound filename>, <off sound filename>"
fn load_handlers(filename: &str) -> Result<HandlerMap, InputError> {
    use std::str::FromStr;

    let file_path = Path::new(filename);

    let mut loaded_sounds: BTreeSet<&String> = BTreeSet::new();

    let input = File::open(filename)?;
    let reader = BufReader::new(input);

    let base_dir = match file_path.parent() {
        Some(p) => p,
        None => return Err(InputError::from_str("Input file does not have a parent")),
    };

    let mut result: HandlerMap = BTreeMap::new();

    for line_result in reader.lines() {
        let line = line_result?;
        let parts: Vec<&str> = line.split(",").collect();

        println!("Got definition for input {:?}", parts);

        assert!(
            parts.len() == 4,
            format!("Incorrect number of elements for {}", line)
        );

        assert!(
            !parts[1].trim().is_empty(),
            format!("Missing name for event: {}", line)
        );

        let key: usize = if parts[0] == "default" {
            simulation::DEFAULT_HANDLER_EVENT
        } else {
            usize::from_str(parts[0])?
        };

        if result.contains_key(&key) {
            warn!("Redefining input: {:?}", parts);
        }

        let on_filename: &String = to_static(parts[2].trim());

        if !on_filename.is_empty() && !loaded_sounds.contains(on_filename) {
            bind_soundfile(&on_filename, &base_dir)?;
            loaded_sounds.insert(on_filename);
        }

        let off_filename: &String = to_static(parts[3].trim());

        if !off_filename.is_empty() && !loaded_sounds.contains(off_filename) {
            bind_soundfile(&off_filename, &base_dir)?;
            loaded_sounds.insert(off_filename);
        }

        if !on_filename.is_empty() || !off_filename.is_empty() {
            let handler_name = to_static(parts[1].trim());

            let handler_func: simulation::HandlerFunc = Box::new(move |value, _| {
                if value == 0 && !off_filename.is_empty() {
                    info!("Playing off sound for {}", handler_name);
                    music::play_sound(&off_filename, music::Repeat::Times(0), music::MAX_VOLUME);
                }

                if value == 1 && !on_filename.is_empty() {
                    info!("Playing on sound for {}", handler_name);
                    music::play_sound(&on_filename, music::Repeat::Times(0), music::MAX_VOLUME);
                }
            });

            let handler = EventHandler::new(handler_name, handler_func);
            result.insert(key, handler);
        }
    }

    Ok(result)
}

use std::path::Path;

fn bind_soundfile(filename: &'static String, base_dir: &Path) -> Result<(), InputError> {
    assert!(!filename.is_empty(), "binding empty filename");

    let provided_path = Path::new(filename);
    let mut absolute_path = base_dir.to_path_buf();

    // Make non-absolute paths relative to the base_dir
    let resolved_path = if provided_path.has_root() {
        provided_path
    } else {
        absolute_path.push(provided_path);
        absolute_path.as_path()
    };

    if !resolved_path.exists() {
        return Err(InputError::new(format!(
            "Sound file '{}' does not exist",
            filename
        )));
    }

    if !resolved_path.is_file() {
        return Err(InputError::new(format!(
            "Sound file '{}' does not exist",
            filename
        )));
    }

    music::bind_sound_file(filename, filename);

    Ok(())
}

fn to_static(input: &str) -> &'static String {
    Box::leak(Box::new(String::from(input)))
}
