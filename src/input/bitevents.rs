use std::fmt;
use std::fmt::Error;
use std::fmt::Formatter;

#[derive(Debug, PartialEq)]
pub struct BitEvent {
    pub bit: usize,
    pub value: u8,
}

impl fmt::Display for BitEvent {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        write!(f, "{{ \"bit {}\": {} }}", self.bit, self.value)
    }
}

pub fn bit_compare(previous: &[u8], current: &[u8], len: usize) -> Vec<BitEvent> {
    let mut current_index = 0;
    let mut events: Vec<BitEvent> = Vec::new();

    // Reverse iteration from LSB to MSB
    for index in (0..len).rev() {
        let delta = previous[index] ^ current[index];
        debug!(
            "{:>0width$b} ^ {:>0width$b} = {:>0width$b}",
            previous[index],
            current[index],
            delta,
            width = 8
        );

        for rotation in 0..8 {
            if bit_value(delta, rotation) != 0 {
                debug!(
                    "  bit {} has changed to {}",
                    current_index,
                    bit_value(current[index], rotation)
                );
                events.push(BitEvent {
                    bit: current_index,
                    value: bit_value(current[index], rotation),
                });
            }
            current_index += 1;
        }
    }

    events
}

fn bit_value(data: u8, shift: usize) -> u8 {
    data >> shift & 0x01
}

#[cfg(test)]
mod tests {
    use crate::input::bitevents::{bit_compare, BitEvent};

    #[test]
    fn confirm_basic_event() {
        let prev: [u8; 2] = [0b11110000, 0b00100100];
        let cur: [u8; 2] = [0b01110001, 0b10100000];

        let events = bit_compare(&prev, &cur, 2);

        assert!(
            events
                == vec!(
                    BitEvent { bit: 2, value: 0 },
                    BitEvent { bit: 7, value: 1 },
                    BitEvent { bit: 8, value: 1 },
                    BitEvent { bit: 15, value: 0 }
                )
        );
    }
}
