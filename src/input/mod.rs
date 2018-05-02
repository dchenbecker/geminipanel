pub mod bitevents;
pub mod mcp23017;
pub mod stdin;

use self::bitevents::BitEvent;

use std::fmt::{Display, Error, Formatter};

#[derive(Debug)]
pub struct InputError {
    message: String,
}

impl Display for InputError {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        write!(f, "InputError(\"{}\")", self.message)
    }
}

pub trait InputHandler {
    fn read_events(&mut self) -> Result<Vec<BitEvent>, InputError>;

    fn set_output(&mut self, dev_index: usize, bits: &[BitEvent]) -> Result<(), InputError>;
}
