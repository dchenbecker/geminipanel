extern crate i2cdev;

#[macro_use]
extern crate log;
extern crate env_logger;

extern crate music;

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

#[derive(Copy, Clone, Hash, PartialEq, Eq)]
enum Sounds {
    TurbineStart,
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
        
        music::set_volume(music::MAX_VOLUME);
        
        info!("Starting music...");
        music::play_music(&Music::Background, music::Repeat::Forever);

        info!("Configuring devices...");

        if cfg!(target_arch = "x86_64") {
            debug!("Read Stdin");
            main_loop(&mut input::stdin::StdinInput::new(), rx, sim);
        } else {
            debug!("Read MCP23017");
            main_loop(&mut input::mcp23017::PanelInputHandler::new(&args).expect("Could not init MCP23017s"), rx, sim);
        }

        info!("Sound complete");
    });
}

fn main_loop<T: input::InputHandler>(input: &mut T, rx: mpsc::Receiver<BitEvent>, sim: simulation::Simulator) {
    loop {
        // fetch any pending handler feedback events
        let feedback_events: Vec<BitEvent> = rx.try_iter().collect();

        if ! feedback_events.is_empty() {
            input.set_output(3, &feedback_events).unwrap_or_else(|err| {
                warn!("Error setting outputs {:?}: {}", feedback_events, err);
            });
        }
        
        match input.read_events() {
            Ok(events) => {
                info!("Read {:?}", events);
                sim.process(&events);
            },
            Err(e) => {
                error!("Error reading events: {}", e);
            }
        }
    }       
}

use std::thread;
use std::time::Duration;

fn init_simulator(sender: &mpsc::Sender<BitEvent>) -> simulation::Simulator {
    use simulation::*;
    
    let handlers = vec![
        EventHandler::new("First handler", |value, _| { info!("Got value of {}", value); }),
        EventHandler::new("Blink test", |value, tx| {
            let output: usize = 3;
            if value != 0 {
                let our_tx = tx.clone();
                thread::spawn(move || {
                    blink(1, output, Duration::from_millis(400), &our_tx)
                });
            } else {
                tx.send(BitEvent { bit: output, value: 0 }).unwrap();
            }
        })
    ];

    Simulator::new(handlers, &sender)
}

/// Blink the given output on/off (one interval each) for the specified count
fn blink(output_id: usize, count: usize, interval: Duration, tx: &mpsc::Sender<BitEvent>) {
    for _ in 0..count {
        tx.send(BitEvent { bit: output_id, value: 1 }).unwrap();
        thread::sleep(interval);
        tx.send(BitEvent { bit: output_id, value: 0 }).unwrap();
        thread::sleep(interval);
    }
    // Finally, turn it on once done blinking
    tx.send(BitEvent { bit: output_id, value: 1 }).unwrap();
}
