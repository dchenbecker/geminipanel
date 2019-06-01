use crate::input::bitevents::BitEvent;
use std::collections::BTreeMap;
use std::sync::mpsc::Sender;

pub const DEFAULT_HANDLER_EVENT: usize = usize::max_value();

pub type HandlerMap = BTreeMap<usize, EventHandler>;

pub struct Simulator {
    handlers: HandlerMap,
    sender: Sender<BitEvent>,
}

impl Simulator {
    pub fn new(handlers: HandlerMap, sender: &Sender<BitEvent>) -> Simulator {
        Simulator {
            handlers,
            sender: (*sender).clone(),
        }
    }

    pub fn process(&self, events: &[BitEvent]) {
        debug!("Processing {} simulation input events", events.len());
        for event in events {
            let target_handler = self
                .handlers
                .get(&event.bit)
                .or(self.handlers.get(&DEFAULT_HANDLER_EVENT));

            if let Some(to_fire) = target_handler {
                info!("Firing '{}' for event {:?}", to_fire.name, event);
                (to_fire.handler)(event.value, &self.sender);
            } else {
                warn!("Event without a handler: {}", event);
            }
        }
    }
}

pub type HandlerFunc = Box<Fn(u8, &Sender<BitEvent>) -> ()>;

pub struct EventHandler {
    name: &'static str,
    handler: HandlerFunc,
}

impl EventHandler {
    pub fn new(name: &'static str, handler: HandlerFunc) -> EventHandler {
        EventHandler { name, handler }
    }
}

// Utility functions
use std::thread;
use std::time::Duration;

/// Blink the given output on/off (one interval each) for the specified count
pub fn blink(output_id: usize, count: usize, interval: Duration, tx: &Sender<BitEvent>) {
    debug!("Blinking {} times with interval {:?}", count, interval);
    for _ in 0..count {
        tx.send(BitEvent {
            bit: output_id,
            value: 1,
        })
        .unwrap();
        thread::sleep(interval);
        tx.send(BitEvent {
            bit: output_id,
            value: 0,
        })
        .unwrap();
        thread::sleep(interval);
    }
    // Finally, turn it on once done blinking
    tx.send(BitEvent {
        bit: output_id,
        value: 1,
    })
    .unwrap();
}
