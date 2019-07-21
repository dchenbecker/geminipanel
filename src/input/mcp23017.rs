use i2cdev::core::*;
use i2cdev::linux::{LinuxI2CDevice, LinuxI2CError};

use std::mem;
use std::thread;
use std::time::Duration;

use super::bitevents::*;
use super::*;

const POLL_TIME: Duration = Duration::from_millis(100);

type ResultBuffer = Box<[u8]>;

struct DeviceConfig<'a> {
    dev_path: &'a str,
    address: u16,
    polarity_mask: u16,
    direction_mask: u16,
}

pub struct PanelInputHandler {
    devices: Vec<MCP23017>,
    curr_buffer: ResultBuffer,
    prev_buffer: ResultBuffer,
}

impl PanelInputHandler {
    pub fn new(argv: &[String]) -> Result<PanelInputHandler, InputError> {
        let devices = setup_devices(&[
            DeviceConfig {
                dev_path: &argv[1],
                address: 0x20,
                polarity_mask: 0x0000,
                direction_mask: 0xffff,
            },
            DeviceConfig {
                dev_path: &argv[1],
                address: 0x21,
                polarity_mask: 0x0000,
                direction_mask: 0xffff,
            },
            DeviceConfig {
                dev_path: &argv[1],
                address: 0x22,
                polarity_mask: 0xfc03,
                direction_mask: 0xffff,
            },
            DeviceConfig {
                dev_path: &argv[2],
                address: 0x20,
                polarity_mask: 0x0000,
                direction_mask: 0x00ff,
            },
        ])?;
        let buffer_len = devices.len() * 2;

        let mut state = PanelInputHandler {
            devices,
            curr_buffer: allocate_slice(buffer_len),
            prev_buffer: allocate_slice(buffer_len),
        };

        // Initial read into previous buffer to set a baseline
        read_devices(&mut state.devices, &mut state.prev_buffer)
            .expect("Failed to read initial state");

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
                BitEvent { bit: 1, value: 1 },
                BitEvent { bit: 3, value: 1 },
                BitEvent { bit: 5, value: 1 },
            ],
        );

        assert!(new_value == 0b0000000000101010);
    }

    #[test]
    fn test_compute_unset_values() {
        let new_value = compute_new_values(
            0xffff,
            &[
                BitEvent { bit: 1, value: 0 },
                BitEvent { bit: 3, value: 0 },
                BitEvent { bit: 5, value: 0 },
            ],
        );

        assert!(new_value == 0b1111111111010101);
    }

    #[test]
    fn test_compute_mixed_values() {
        let new_value = compute_new_values(
            0x00ff,
            &[
                BitEvent { bit: 1, value: 0 },
                BitEvent { bit: 3, value: 1 },
                BitEvent { bit: 9, value: 0 },
                BitEvent { bit: 11, value: 1 },
            ],
        );

        println!("{:016b} => {:016b}", 0x00ff, new_value);
        assert!(new_value == 0b0000100011111101);
    }
}

impl InputHandler for PanelInputHandler {
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
    dev: LinuxI2CDevice,
    address: u16,
}

impl MCP23017 {
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

pub fn setup_mcp23017(
    device_path: &str,
    address: u16,
    polarity_mask: u16,
    direction_mask: u16,
) -> Result<MCP23017, LinuxI2CError> {
    let mut dev = LinuxI2CDevice::new(device_path, address)?;

    // Set the IO Direction registers. Also enable the pullup on any input pins
    dev.smbus_write_word_data(IODIR, direction_mask)?;
    dev.smbus_write_word_data(GPPU, direction_mask)?;

    // Set the polarity mask
    dev.smbus_write_word_data(IPOL, !polarity_mask)?;

    Ok(MCP23017 { dev, address })
}

fn setup_devices(devices: &[DeviceConfig]) -> Result<Vec<MCP23017>, LinuxI2CError> {
    devices
        .iter()
        .map(|config| {
            setup_mcp23017(
                &config.dev_path,
                config.address,
                config.polarity_mask,
                config.direction_mask,
            )
        })
        .collect()
}

fn allocate_slice(len: usize) -> ResultBuffer {
    info!("Allocating buffer of size {}", len);
    vec![0; len].into_boxed_slice()
}

fn poll_inputs(state: &mut PanelInputHandler) -> Result<Vec<BitEvent>, InputError> {
    loop {
        read_devices(&mut state.devices, &mut state.curr_buffer)?;

        debug!("Current: {:?}", state.curr_buffer);
        debug!("Prev   : {:?}", state.prev_buffer);

        let events = bit_compare(
            &state.prev_buffer,
            &state.curr_buffer,
            state.curr_buffer.len(),
        );

        // Make current buffer previous for the next cycle
        mem::swap(&mut state.prev_buffer, &mut state.curr_buffer);

        if !events.is_empty() {
            return Ok(events);
        }

        thread::sleep(POLL_TIME);
    }
}

fn read_devices(devices: &mut Vec<MCP23017>, buffer: &mut [u8]) -> Result<(), LinuxI2CError> {
    let dev_len = devices.len();

    debug!("Assert buffer ({}) == {}", buffer.len(), dev_len * 2);

    assert!(buffer.len() == (dev_len * 2));

    devices
        .iter_mut()
        .zip(0..dev_len)
        .map(|(dev, index)| {
            // index is the u16 index, so we need to multiply by two for the u8 base index
            let buffer_base = index << 1;
            dev.read_pins().map(|value| {
                buffer[buffer_base] = (value & 0xff) as u8;
                buffer[buffer_base + 1] = ((value >> 8) & 0xff) as u8;
            })
        })
        .collect()
}
