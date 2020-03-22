use std::fmt;
use std::fmt::Error;
use std::fmt::Formatter;

use actix::prelude::*;

#[derive(Clone, Debug, PartialEq, Message)]
#[rtype(result = "()")]
pub struct BitEvent {
    pub dev_name: String,
    pub bit: u8,
    pub value: u8,
}

impl fmt::Display for BitEvent {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        write!(
            f,
            "{{ dev: {}, bit {}: {} }}",
            self.dev_name, self.bit, self.value
        )
    }
}

pub fn bit_compare(dev_name: &str, previous: u16, current: u16) -> Vec<BitEvent> {
    let mut events: Vec<BitEvent> = Vec::new();

    let delta = previous ^ current;

    debug!(
        "{:>0width$b} ^ {:>0width$b} = {:>0width$b}",
        previous,
        current,
        delta,
        width = 16
    );

    for bit_index in 0u8..16u8 {
        if bit_value(delta, bit_index) != 0 {
            debug!(
                "  bit {} has changed to {}",
                bit_index,
                bit_value(current, bit_index)
            );

            events.push(BitEvent {
                dev_name: String::from(dev_name),
                bit: bit_index,
                value: bit_value(current, bit_index),
            });
        }
    }

    events
}

fn bit_value(data: u16, shift: u8) -> u8 {
    (data >> shift & 0x01) as u8
}

#[cfg(test)]
mod tests {
    use crate::input::bitevents::{bit_compare, BitEvent};

    #[test]
    fn confirm_basic_event() {
        let prev: u16 = 0b1111000000100100;
        let cur: u16 = 0b0111000110100000;

        let events = bit_compare("test", prev, cur);

        assert!(
            events
                == vec!(
                    BitEvent {
                        dev_name: String::from("test"),
                        bit: 2,
                        value: 0
                    },
                    BitEvent {
                        dev_name: String::from("test"),
                        bit: 7,
                        value: 1
                    },
                    BitEvent {
                        dev_name: String::from("test"),
                        bit: 8,
                        value: 1
                    },
                    BitEvent {
                        dev_name: String::from("test"),
                        bit: 15,
                        value: 0
                    }
                )
        );
    }
}
