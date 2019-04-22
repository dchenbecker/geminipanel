pub mod bitevents;
pub mod mcp23017;
pub mod stdin;

use self::bitevents::BitEvent;

use std::fmt::{Display, Error, Formatter};
use std::io;

#[derive(Debug)]
pub struct InputError {
    message: String,
}

impl InputError {
    pub fn new(message: String) -> InputError {
        InputError { message }
    }

    pub fn from_str(message: &str) -> InputError {
        InputError::new(message.to_string())
    }
}

impl Display for InputError {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        write!(f, "InputError(\"{}\")", self.message)
    }
}

impl From<io::Error> for InputError {
    fn from(err: io::Error) -> InputError {
        InputError {
            message: format!("Error: {}", err),
        }
    }
}

use std::num::ParseIntError;

impl From<ParseIntError> for InputError {
    fn from(err: ParseIntError) -> InputError {
        InputError {
            message: format!("Error: {}", err),
        }
    }
}

pub trait InputHandler {
    fn read_events(&mut self) -> Result<Vec<BitEvent>, InputError>;

    fn set_output(&mut self, dev_index: usize, bits: &[BitEvent]) -> Result<(), InputError>;

    fn shutdown(self);
}
