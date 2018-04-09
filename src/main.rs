extern crate i2cdev;

#[macro_use]
extern crate log;
extern crate env_logger;

extern crate music;

use std::env;
use std::process;

mod input;
mod simulation;

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

    if args.len() != 2 {
        eprintln!("Usage: {} <i2c dev path>", args[0]);
        process::exit(-1);
    }

    env_logger::init();

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
            main_loop(&mut input::stdin::StdinInput::new());
        } else {
            debug!("Read MCP23017");
            main_loop(&mut input::mcp23017::PanelInputHandler::new(args).expect("Could not init MCP23017s"));
        }
        
        info!("Sound complete");
    });
}

fn main_loop<T: input::InputHandler>(input: &mut T) {
    loop {
        match input.read_events() {
            Ok(events) => {
                info!("Read {:?}", events);
            },
            Err(e) => {
                error!("Error reading events: {}", e);
            }
        }
    }       
}
