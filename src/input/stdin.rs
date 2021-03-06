use std::io;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, SendError, Sender, TryRecvError};
use std::sync::Arc;
use std::thread;
use std::thread::JoinHandle;

use super::bitevents::BitEvent;
use super::*;

pub struct StdinInput {
    poller: JoinHandle<()>,
    rx: Receiver<String>,
    poll_condition: Arc<AtomicBool>,
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

        let poll_guard = Arc::new(AtomicBool::new(true));

        let poll_condition = poll_guard.clone();

        let poller = thread::spawn(move || {
            let mut buffer = String::with_capacity(DEFAULT_INPUT_BUFFER_SIZE);
            let should_run = poll_guard.clone();
            while should_run.load(Ordering::Relaxed) {
                buffer.clear();
                read_and_send(&mut buffer, &mut tx).unwrap_or_else(|err| {
                    warn!("Error polling stdin: {}", err.message);
                });
            }
        });

        // Noop
        StdinInput {
            poller,
            rx,
            poll_condition,
        }
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
                        if parts.len() == 3 {
                            Ok(BitEvent {
                                dev_name: String::from(parts[0]),
                                bit: parts[1].parse()?,
                                value: parts[2].parse()?,
                            })
                        } else {
                            Err(InputError {
                                message: format!("Invalid input spec: '{}'", s),
                            })
                        }
                    })
                    .collect()
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
        println!("Setting bits {:?} for dev {}", bits, dev_index);
        Ok(())
    }

    fn shutdown(self) {
        debug!("Shutting down STDIN poller thread");

        self.poll_condition.store(false, Ordering::Relaxed);
        self.poller
            .join()
            .expect("Failed to cleanly shut down StdinInput");
    }
}
