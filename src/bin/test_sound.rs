extern crate music;
use std::thread::sleep;
use std::time::Duration;

#[derive(Copy, Clone, Hash, PartialEq, Eq)]
enum Music {
    Background,
}

#[derive(Copy, Clone, Hash, PartialEq, Eq)]
enum Sounds {
    TurbineStart,
}

fn main() {
    music::start::<Music, Sounds, _>(16, || {
        music::bind_music_file(Music::Background, "./sounds/background.wav");
        music::bind_sound_file(Sounds::TurbineStart, "./sounds/turbine_startup_fade.wav");

        music::set_volume(music::MAX_VOLUME);

        println!("Starting music...");
        music::play_music(&Music::Background, music::Repeat::Forever);

        sleep(Duration::from_secs(2));
    })
}
