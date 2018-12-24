use std::io;
use std::num::ParseIntError;
use std::sync::mpsc::{channel, Receiver, SendError, Sender, TryRecvError};
use std::thread;
use std::thread::JoinHandle;

use super::bitevents::BitEvent;
use super::*;

pub struct StdinInput {
    poller: JoinHandle<()>,
    rx: Receiver<String>,
}

impl From<io::Error> for InputError {
    fn from(err: io::Error) -> InputError {
        InputError {
            message: format!("Error: {}", err),
        }
    }
}

impl From<ParseIntError> for InputError {
    fn from(err: ParseIntError) -> InputError {
        InputError {
            message: format!("Error: {}", err),
        }
    }
}

impl From<SendError<String>> for InputError {
    fn from(err: SendError<String>) -> InputError {
        InputError {
            message: format!("Unable to send '{}' on channel", err.0),
        }
    }
}

const DEFAULT_INPUT_BUFFER_SIZE: usize = 1024;

impl StdinInput {
    pub fn new() -> StdinInput {
        // Set up a thread to poll for input on stdin, and a channel to use for transferring that input
        let (mut tx, rx) = channel();

        let poller = thread::spawn(move || {
            let mut buffer = String::with_capacity(DEFAULT_INPUT_BUFFER_SIZE);
            loop {
                buffer.clear();
                read_and_send(&mut buffer, &mut tx).unwrap_or_else(|err| {
                    warn!("Error polling stdin: {}", err.message);
                });
            }
        });

        // Noop
        StdinInput { poller, rx }
    }
}

/// A utility method to simplify error type unification
fn read_and_send(mut buffer: &mut String, tx: &mut Sender<String>) -> Result<(), InputError> {
    io::stdin().read_line(&mut buffer)?;

    let trimmed = buffer.trim();

    if !trimmed.is_empty() {
        Ok(tx.send(trimmed.clone().to_string())?)
    } else {
        // Empty input is not an error
        Ok(())
    }
}

impl InputHandler for StdinInput {
    fn read_events(&mut self) -> Result<Vec<BitEvent>, InputError> {
        match self.rx.try_recv() {
            Ok(input) => {
                // The poller thread guarantees that input is non-empty
                input
                    .split(',')
                    .map(|s| {
                        debug!("Parsing '{}'", s);
                        let parts = s.split(':').collect::<Vec<_>>();
                        if parts.len() == 2 {
                            Ok(BitEvent {
                                bit: parts[0].parse()?,
                                value: parts[1].parse()?,
                            })
                        } else {
                            Err(InputError {
                                message: format!("Invalid input spec: '{}'", s),
                            })
                        }
                    }).collect()
            }

            Err(TryRecvError::Empty) => {
                // Noop, empty input is fine
                Ok(vec![])
            }

            Err(TryRecvError::Disconnected) => {
                panic!("Stdin channel is disconnected! Hard stop.");
            }
        }
    }

    fn set_output(&mut self, dev_index: usize, bits: &[BitEvent]) -> Result<(), InputError> {
        Ok(println!("Setting bits {:?} for dev {}", bits, dev_index))
    }

    fn shutdown(self) {
        debug!("Shutting down STDIN poller thread");

        self.poll_condition.store(false, Ordering::Relaxed);
        self.poller
            .join()
            .expect("Failed to cleanly shut down StdinInput");
    }
}
