extern crate i2cdev;
#[macro_use]
extern crate maplit;

#[macro_use]
extern crate log;
extern crate env_logger;

extern crate music;

use std::collections::BTreeSet;
use std::env;
use std::process;
use std::sync::atomic;
use std::sync::mpsc;

mod input;
mod simulation;

use input::bitevents::BitEvent;

#[derive(Copy, Clone, Hash, PartialEq, Eq)]
enum Music {
    Background,
}

#[derive(Copy, Clone, Hash, PartialEq, Eq)]
enum Sounds {
    TurbineStart,
    TurbineShutdown,
    Launch,
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 3 || args.len() > 4 {
        eprintln!(
            "Usage: {} <i2c dev path 1> <i2c dev path 2> [<event handler file>]",
            args[0]
        );
        process::exit(-1);
    }

    env_logger::init();

    // Set up a channel for simulation feedback
    let (tx, rx) = mpsc::channel::<BitEvent>();

    let handlers = if args.len() == 4 {
        load_handlers(&args[3]).expect("Failed to load handlers")
    } else {
        test_handlers()
    };

    let sim = init_simulator(&tx, handlers);

    info!("Init sound...");

    music::start::<Music, Sounds, _>(16, || {
        music::bind_music_file(Music::Background, "./sounds/background.wav");
        music::bind_sound_file(Sounds::TurbineStart, "./sounds/turbine_startup_fade.wav");
        music::bind_sound_file(
            Sounds::TurbineShutdown,
            "./sounds/turbine_startup_fade_reverse.wav",
        );
        music::bind_sound_file(Sounds::Launch, "./sounds/full-launch.wav");

        music::set_volume(music::MAX_VOLUME);

        info!("Starting music...");
        music::play_music(&Music::Background, music::Repeat::Forever);

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
static TURBINES_ON: atomic::AtomicBool = atomic::AtomicBool::new(false);

use std::thread;
use std::time::Duration;

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

    let mut loaded_sounds: BTreeSet<&String> = BTreeSet::new();

    let input = File::open(filename)?;
    let reader = BufReader::new(input);

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
            bind_soundfile(&on_filename)?;
            loaded_sounds.insert(on_filename);
        }

        let off_filename: &String = to_static(parts[3].trim());

        if !off_filename.is_empty() && !loaded_sounds.contains(off_filename) {
            bind_soundfile(&off_filename)?;
            loaded_sounds.insert(off_filename);
        }

        if !on_filename.is_empty() || !off_filename.is_empty() {
            let handler_name = to_static(parts[1].trim());

            let handler_func: simulation::HandlerFunc = Box::new(move |value, _| {
                if value == 0 && !off_filename.is_empty() {
                    debug!("Playing off sound for {}", handler_name);
                    music::play_sound(&off_filename, music::Repeat::Times(0), music::MAX_VOLUME);
                }

                if value == 1 && !on_filename.is_empty() {
                    debug!("Playing on sound for {}", handler_name);
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

fn bind_soundfile(filename: &'static String) -> Result<(), InputError> {
    assert!(!filename.is_empty(), "binding empty filename");

    let p = Path::new(filename);

    if !p.exists() {
        return Err(InputError::new(format!(
            "Sound file '{}' does not exist",
            filename
        )));
    }

    if !p.is_file() {
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

fn test_handlers() -> HandlerMap {
    use simulation::blink;

    btreemap! {
        0 => EventHandler::new("First handler", Box::new(move |value, _| { info!("Got value of {} in handler", value); })),
        1 => EventHandler::new("Blink test", Box::new(move |value, tx| {
            const OUTPUT_PIN :usize = 1;
            let output: usize = 3;
            if value != 0 {
                let our_tx = tx.clone();
                thread::spawn(move || {
                    debug!("Invoking blink");
                    blink(OUTPUT_PIN, output, Duration::from_millis(400), &our_tx)
                });
            } else {
                tx.send(BitEvent { bit: OUTPUT_PIN, value: 0 }).unwrap();
            }
        })),
        2 => EventHandler::new("Turbine control", Box::new(move |value, _| {
            if value == 0 {
                if TURBINES_ON.load(atomic::Ordering::Relaxed) {
                    debug!("Stopping turbine.");
                    TURBINES_ON.store(false, atomic::Ordering::Relaxed);
                    music::play_sound(&Sounds::TurbineShutdown, music::Repeat::Times(0), music::MAX_VOLUME);
                } else {
                    debug!("Turbines already off. NOOP");
                }
            } else {
                if ! TURBINES_ON.load(atomic::Ordering::Relaxed) {
                    debug!("Starting turbine.");
                    TURBINES_ON.store(true, atomic::Ordering::Relaxed);
                    music::play_sound(&Sounds::TurbineStart, music::Repeat::Times(0), music::MAX_VOLUME);
                } else {
                    debug!("Turbines already started. NOOP");
                }
            }
        }))
    }
}
