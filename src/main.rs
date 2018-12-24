extern crate i2cdev;
#[macro_use]
extern crate maplit;

#[macro_use]
extern crate log;
extern crate env_logger;

extern crate music;

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

    if args.len() != 3 {
        eprintln!("Usage: {} <i2c dev path 1> <i2c dev path 2>", args[0]);
        process::exit(-1);
    }

    env_logger::init();

    // Set up a channel for simulation feedback
    let (tx, rx) = mpsc::channel::<BitEvent>();

    let sim = init_simulator(&tx);

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

        if cfg!(target_arch = "x86_64") {
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

use std::thread;
use std::time::Duration;

// Globals for now, need to encapsulate state later
static TURBINES_ON: atomic::AtomicBool = atomic::AtomicBool::new(false);

fn init_simulator(sender: &mpsc::Sender<BitEvent>) -> simulation::Simulator {
    use simulation::*;

    let handlers = btreemap!{
        0 => EventHandler::new("First handler", |value, _| { info!("Got value of {} in handler", value); }),
        1 => EventHandler::new("Blink test", |value, tx| {
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
        }),
        2 => EventHandler::new("Turbine control", |value, _| {
            if value == 0 {
                debug!("Stopping turbine.");
                TURBINES_ON.store(false, atomic::Ordering::Relaxed);
                music::play_sound(&Sounds::TurbineShutdown, music::Repeat::Times(0), music::MAX_VOLUME);
            } else {
                debug!("Starting turbine.");
                TURBINES_ON.store(true, atomic::Ordering::Relaxed);
                music::play_sound(&Sounds::TurbineStart, music::Repeat::Times(0), music::MAX_VOLUME);
            }
        })
    };

    Simulator::new(handlers, &sender)
}

/// Blink the given output on/off (one interval each) for the specified count
fn blink(output_id: usize, count: usize, interval: Duration, tx: &mpsc::Sender<BitEvent>) {
    debug!("Blinking {} times with interval {:?}", count, interval);
    for _ in 0..count {
        tx.send(BitEvent {
            bit: output_id,
            value: 1,
        }).unwrap();
        thread::sleep(interval);
        tx.send(BitEvent {
            bit: output_id,
            value: 0,
        }).unwrap();
        thread::sleep(interval);
    }
    // Finally, turn it on once done blinking
    tx.send(BitEvent {
        bit: output_id,
        value: 1,
    }).unwrap();
}
