use i2cdev::core::*;
use i2cdev::linux::{LinuxI2CDevice, LinuxI2CError};

use std::thread;
use std::time::Duration;

use super::bitevents::*;
use super::*;

pub mod config;
use crate::input::mcp23017::config::DeviceConfig;

const POLL_TIME: Duration = Duration::from_millis(100);

pub struct PanelInputHandler {
    // Input is handled as a pair of device and previous state
    devices: Vec<MCP23017>,
}

impl PanelInputHandler {
    pub fn new(device_config: &[DeviceConfig]) -> Result<PanelInputHandler, InputError> {
        let devices = setup_devices(device_config)?;

        let state = PanelInputHandler { devices };

        Ok(state)
    }
}

fn compute_new_values(current_value: u16, bits: &[BitEvent]) -> u16 {
    bits.iter().fold(current_value, |value, event| {
        let raw_mask: u16 = 1 << event.bit;

        let mask = if event.value == 0 {
            !raw_mask
        } else {
            raw_mask
        };

        if event.value != 0 {
            value | mask
        } else {
            value & mask
        }
    })
}

#[cfg(test)]
mod tests {
    use crate::input::bitevents::BitEvent;
    use crate::input::mcp23017::compute_new_values;

    #[test]
    fn test_compute_set_values() {
        let new_value = compute_new_values(
            0,
            &[
                BitEvent {
                    dev_name: String::from("test"),
                    bit: 1,
                    value: 1,
                },
                BitEvent {
                    dev_name: String::from("test"),
                    bit: 3,
                    value: 1,
                },
                BitEvent {
                    dev_name: String::from("test"),
                    bit: 5,
                    value: 1,
                },
            ],
        );

        assert!(new_value == 0b0000000000101010);
    }

    #[test]
    fn test_compute_unset_values() {
        let new_value = compute_new_values(
            0xffff,
            &[
                BitEvent {
                    dev_name: String::from("test"),
                    bit: 1,
                    value: 0,
                },
                BitEvent {
                    dev_name: String::from("test"),
                    bit: 3,
                    value: 0,
                },
                BitEvent {
                    dev_name: String::from("test"),
                    bit: 5,
                    value: 0,
                },
            ],
        );

        assert!(new_value == 0b1111111111010101);
    }

    #[test]
    fn test_compute_mixed_values() {
        let new_value = compute_new_values(
            0x00ff,
            &[
                BitEvent {
                    dev_name: String::from("test"),
                    bit: 1,
                    value: 0,
                },
                BitEvent {
                    dev_name: String::from("test"),
                    bit: 3,
                    value: 1,
                },
                BitEvent {
                    dev_name: String::from("test"),
                    bit: 9,
                    value: 0,
                },
                BitEvent {
                    dev_name: String::from("test"),
                    bit: 11,
                    value: 1,
                },
            ],
        );

        println!("{:016b} => {:016b}", 0x00ff, new_value);
        assert!(new_value == 0b0000100011111101);
    }
}

impl<'config> InputHandler for PanelInputHandler {
    fn read_events(&mut self) -> Result<Vec<BitEvent>, InputError> {
        poll_inputs(self)
    }

    fn set_output(&mut self, dev_index: usize, bits: &[BitEvent]) -> Result<(), InputError> {
        if dev_index >= self.devices.len() {
            Err(InputError {
                message: format!(
                    "Invalid device index {}. Number of devices {}",
                    dev_index,
                    self.devices.len()
                ),
            })
        } else {
            let dev: &mut MCP23017 = &mut self.devices[dev_index];
            if bits.iter().any(|event| event.bit > 15) {
                Err(InputError {
                    message: format!("Invalid bit events: {:?}", bits),
                })
            } else {
                let new_value: u16 = compute_new_values((*dev).read_pins()?, bits);
                Ok((*dev).write_pins(new_value)?) // I hate this pattern
            }
        }
    }

    fn shutdown(self) {
        debug!("Shutdown is NOOP on MCP23017");
    }
}

impl From<LinuxI2CError> for InputError {
    fn from(err: LinuxI2CError) -> InputError {
        InputError {
            message: format!("I2C Error: {}", err),
        }
    }
}

// Important registers, assuming IOCON.BANK == 0
const IODIR: u8 = 0x00;
const IPOL: u8 = 0x02;
const GPPU: u8 = 0x0C;
const GPIO: u8 = 0x12;

pub struct MCP23017 {
    dev_name: String,
    dev: LinuxI2CDevice,
    address: u16,
    previous_value: u16,
}

impl MCP23017 {
    pub fn poll_input(&mut self) -> Result<Vec<BitEvent>, LinuxI2CError> {
        let new_value = self.read_pins()?;

        debug!("Current for {} : {:#x}", self.dev_name, new_value);
        debug!("Prev for {}    : {:#x}", self.dev_name, self.previous_value);

        let events = bit_compare(&self.dev_name, self.previous_value, new_value);
        self.previous_value = new_value;
        Ok(events)
    }

    pub fn read_pins(&mut self) -> Result<u16, LinuxI2CError> {
        let mut result = u16::from(self.dev.smbus_read_byte_data(GPIO)?);
        result |= u16::from(self.dev.smbus_read_byte_data(GPIO + 1)?) << 8;
        debug!("Read 0x{:04x} from 0x{:02x}", result, self.address);
        Ok(result)
    }

    pub fn write_pins(&mut self, new_values: u16) -> Result<(), LinuxI2CError> {
        self.dev.smbus_write_word_data(GPIO, new_values)
    }
}

pub fn setup_mcp23017(config: &DeviceConfig) -> Result<MCP23017, LinuxI2CError> {
    let mut dev = LinuxI2CDevice::new(&config.dev_path, config.address)?;

    // Set the IO Direction registers. Also enable the pullup on any input pins
    dev.smbus_write_word_data(IODIR, config.direction_mask)?;
    dev.smbus_write_word_data(GPPU, config.direction_mask)?;

    // Set the polarity mask
    dev.smbus_write_word_data(IPOL, !config.polarity_mask)?;

    let mut dev = MCP23017 {
        dev_name: config.dev_name.clone(),
        dev,
        address: config.address,
        previous_value: 0,
    };

    // Perform a read to get the initial value
    let current_value = dev.read_pins()?;
    dev.previous_value = current_value;
    Ok(dev)
}

fn setup_devices(devices: &[DeviceConfig]) -> Result<Vec<MCP23017>, LinuxI2CError> {
    devices.iter().map(&setup_mcp23017).collect()
}

fn poll_inputs(state: &mut PanelInputHandler) -> Result<Vec<BitEvent>, InputError> {
    loop {
        let mut events: Vec<BitEvent> = Vec::new();

        for dev in &mut state.devices {
            let mut inputs = dev.poll_input()?;
            events.append(&mut inputs);
        }

        if !events.is_empty() {
            return Ok(events);
        }

        thread::sleep(POLL_TIME);
    }
}
