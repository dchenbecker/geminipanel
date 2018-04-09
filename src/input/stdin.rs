use std::io;
use std::num::ParseIntError;

use super::*;
use super::bitevents::BitEvent;

pub struct StdinInput {
    input_buffer: String
}

impl From<io::Error> for InputError {
    fn from(err: io::Error) -> InputError {
        InputError { message: format!("Error: {}", err) }
    }
}

impl From<ParseIntError> for InputError {
    fn from(err: ParseIntError) -> InputError {
        InputError { message: format!("Error: {}", err) }
    }
}

impl StdinInput {
    pub fn new() -> StdinInput {
        // Noop
        StdinInput { input_buffer: String::new() }
    }
}


impl InputHandler for StdinInput {
    fn read_events(&mut self) -> Result<Vec<BitEvent>, InputError> {
        self.input_buffer.clear();
        
        io::stdin().read_line(&mut self.input_buffer)?;
        
        self.input_buffer.trim().split(',').map(|s| {
            debug!("Parsing '{}'", s);
            let parts = s.split(':').collect::<Vec<_>>();
            if parts.len() == 2 {
                Ok(BitEvent { bit: parts[0].parse()?, value: parts[1].parse()? })
            } else {
                Err(InputError { message: format!("Invalid input spec: '{}'", s) })
            }
        }).collect()
    }
}
