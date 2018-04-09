extern crate i2cdev;

use i2cdev::core::*;
use i2cdev::linux::LinuxI2CDevice;

use std::env;
use std::process;

const IODIR: u8 = 0x00;
const IPOL: u8 = 0x02;
const GPPU: u8 = 0x0c;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() != 3 {
        eprintln!("Usage: {} <i2c dev path> <address>", &args[0]);
        process::exit(-1);
    }

    println!("Configuring device...");

    let address = u16::from_str_radix(&args[2], 16).expect("Invalid address specified");
    let mut dev = LinuxI2CDevice::new(&args[1], address).expect("Could not open device");

    println!("Setting registers...");

    dev.smbus_write_word_data(IODIR, 0xFFFF).expect("Failed to set IODIR");
    dev.smbus_write_word_data(IPOL, 0xFFFF).expect("Failed to set IPOL");
    dev.smbus_write_word_data(GPPU, 0xFFFF).expect("Failed to set GPPU");
}
        
