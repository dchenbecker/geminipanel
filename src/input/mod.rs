mod bitevents;

pub mod mcp23017;
pub mod stdin;

use std::fmt::{Display, Error, Formatter};

#[derive(Debug)]
pub struct InputError {
    message: String
}

impl Display for InputError {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        write!(f, "InputError(\"{}\")", self.message)
    }
}

pub trait InputHandler {
    fn read_events(&mut self) -> Result<Vec<bitevents::BitEvent>, InputError>;
}
