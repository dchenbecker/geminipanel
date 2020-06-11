extern crate i2cdev;

extern crate serde;
extern crate serde_yaml;

#[macro_use]
extern crate log;
use env_logger;

use sdl2::mixer;

use actix::prelude::*;

use std::env;
use std::io::Read;
use std::process;

mod bindfiles;
mod input;
mod simulation;
mod utility;

const REPEAT_FOREVER: i32 = -1;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 || args.len() > 3 {
        eprintln!("Usage: {} <device config> <event handler file>", args[0]);
        process::exit(-1);
    }

    env_logger::init();

    // Initiate the Actix system
    let system = System::new("gemini");

    info!("Init sound...");

    let _ = sdl2::init().unwrap();
    let _ = mixer::init(mixer::InitFlag::MP3).unwrap();
    mixer::open_audio(
        mixer::DEFAULT_FREQUENCY,
        mixer::DEFAULT_FORMAT,
        mixer::DEFAULT_CHANNELS,
        1024,
    )
    .unwrap();
    mixer::allocate_channels(16);

    let background = mixer::Music::from_file("./sounds/background.wav").unwrap();
    mixer::Music::set_volume(mixer::MAX_VOLUME);
    background.play(REPEAT_FOREVER);

    info!("Sound complete");

    let handlers = load_handlers(&args[2]).expect("Failed to load handlers");

    let sim = simulation::Simulator::new(handlers).start();

    info!("Configuring devices...");

    if args[1].to_lowercase() == "stdin" {
        debug!("Read Stdin");

        input::stdin::StdinInput {
            recipient: sim.recipient(),
        }
        .start();
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

        input::mcp23017::PanelInputHandler::new(&devices, sim.recipient())
            .unwrap()
            .start();
    }

    system.run().unwrap();
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
        let off_file = bindfiles::parse_sound_filename(parts[3].trim()).unwrap();

        if let Some(handler) =
            bindfiles::create_handler(to_static(parts[2].trim()), on_file, off_file)?
        {
            result.insert(key, handler);
        }
    }

    Ok(result)
}

use std::path::Path;

fn to_static(input: &str) -> &'static String {
    Box::leak(Box::new(String::from(input)))
}
