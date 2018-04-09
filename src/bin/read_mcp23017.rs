extern crate i2cdev;

use i2cdev::core::*;
use i2cdev::linux::LinuxI2CDevice;

use std::env;
use std::process;

const REGISTER_COUNT: u8 = 0x15;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() != 3 {
        eprintln!("Usage: {} <i2c dev path> <address>", &args[0]);
        process::exit(-1);
    }

    println!("Configuring device...");

    let address = u16::from_str_radix(&args[2], 16).expect("Invalid address specified");
    let mut dev = LinuxI2CDevice::new(&args[1], address).expect("Could not open device");

    match dev.smbus_read_i2c_block_data(0x00, REGISTER_COUNT) {
        Ok(data) => {
            println!("Read values:");
            (0..REGISTER_COUNT).zip(data).for_each(|(index,value)| {
                println!("  {:02x}: {:02x}", index, value);
            });
        }
        Err(e) => eprintln!("Error reading: {}", e)
    }
}
        
