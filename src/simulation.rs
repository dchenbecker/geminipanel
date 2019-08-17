use crate::input::bitevents::BitEvent;
use std::collections::BTreeMap;
use std::sync::mpsc::Sender;

pub fn default_handler_event() -> (String, u8) {
    (String::from(crate::DEFAULT_NAME), 0)
}

// Map a device name and bit number to the handler
pub type HandlerMap = BTreeMap<(String, u8), EventHandler>;

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
                .get(&(event.dev_name.clone(), event.bit))
                .or(self.handlers.get(&default_handler_event()));

            if let Some(to_fire) = target_handler {
                info!("Firing '{}' for event {:?}", to_fire.name, event);
                (to_fire.handler)(event.value, &self.sender);
            } else {
                warn!("Event without a handler: {}", event);
            }
        }
    }
}

pub type HandlerFunc = Box<dyn Fn(u8, &Sender<BitEvent>) -> ()>;

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
pub fn blink(
    dev_name: &str,
    output_id: u8,
    count: usize,
    interval: Duration,
    tx: &Sender<BitEvent>,
) {
    debug!("Blinking {} times with interval {:?}", count, interval);
    for _ in 0..count {
        tx.send(BitEvent {
            dev_name: String::from(dev_name),
            bit: output_id,
            value: 1,
        })
        .unwrap();
        thread::sleep(interval);
        tx.send(BitEvent {
            dev_name: String::from(dev_name),
            bit: output_id,
            value: 0,
        })
        .unwrap();
        thread::sleep(interval);
    }
    // Finally, turn it on once done blinking
    tx.send(BitEvent {
        dev_name: String::from(dev_name),
        bit: output_id,
        value: 1,
    })
    .unwrap();
}
