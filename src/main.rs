extern crate i2cdev;

extern crate serde;
extern crate serde_yaml;

#[macro_use]
extern crate log;
extern crate env_logger;

extern crate music;

use std::collections::BTreeSet;
use std::env;
use std::io::Read;
use std::process;
use std::sync::mpsc;

mod bindfiles;
mod input;
mod simulation;

use input::bitevents::BitEvent;

#[derive(Copy, Clone, Hash, PartialEq, Eq)]
enum Music {
    Background,
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 || args.len() > 3 {
        eprintln!("Usage: {} <device config> <event handler file>", args[0]);
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

        let handlers = load_handlers(&args[2]).expect("Failed to load handlers");

        let sim = init_simulator(&tx, handlers);

        info!("Configuring devices...");

        if args[1].to_lowercase() == "stdin" {
            debug!("Read Stdin");
            main_loop(&mut input::stdin::StdinInput::new(), rx, sim);
        } else {
            debug!("Read MCP23017");

            // Read in device config
            let mut dev_config_contents = String::new();
            File::open(args[1].clone())
                .unwrap()
                .read_to_string(&mut dev_config_contents)
                .unwrap();

            let devices: Vec<input::mcp23017::config::DeviceConfig> =
                serde_yaml::from_str(&dev_config_contents).unwrap();

            println!("Read devices: {:?}", devices);

            main_loop(
                &mut input::mcp23017::PanelInputHandler::new(&devices)
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
use simulation::HandlerMap;
use std::collections::BTreeMap;
use std::fs::File;
use std::io::{BufRead, BufReader};

const DEFAULT_NAME: &str = "default";

// Format for each line is "<device name>, <input index>, <name>, <on sound filename>, <off sound filename>"
// Filenames can have an optional float suffix (0-1] to specify volume
fn load_handlers(filename: &str) -> Result<HandlerMap, InputError> {
    use std::str::FromStr;

    let file_path = Path::new(filename);

    let mut loaded_sounds: BTreeSet<&String> = BTreeSet::new();

    let input = File::open(filename).unwrap();
    let reader = BufReader::new(input);

    let base_dir = match file_path.parent() {
        Some(p) => p,
        None => return Err(InputError::from_str("Input file does not have a parent")),
    };

    let mut result: HandlerMap = BTreeMap::new();

    for line_result in reader.lines() {
        let line = line_result.unwrap();
        let parts = bindfiles::split_sound_line(&line).unwrap();

        let key: (String, u8) = if parts[0] == DEFAULT_NAME {
            debug!("Default handler: {}", line);
            simulation::default_handler_event()
        } else {
            (String::from(parts[0]), u8::from_str(parts[1]).unwrap())
        };

        if result.contains_key(&key) {
            warn!("Redefining input: {:?}", parts);
        }

        let on_file = bindfiles::parse_sound_filename(parts[3].trim()).unwrap();

        if let Some((on_filename, _)) = on_file {
            if !loaded_sounds.contains(on_filename) {
                bind_soundfile(&on_filename, &base_dir).unwrap();
                loaded_sounds.insert(on_filename);
            }
        }

        let off_file = bindfiles::parse_sound_filename(parts[3].trim()).unwrap();

        if let Some((off_filename, _)) = off_file {
            if !loaded_sounds.contains(off_filename) {
                bind_soundfile(&off_filename, &base_dir).unwrap();
                loaded_sounds.insert(off_filename);
            }
        }

        if let Some(handler) =
            bindfiles::create_handler(to_static(parts[1].trim()), on_file, off_file)
        {
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
