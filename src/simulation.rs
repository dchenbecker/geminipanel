use crate::input::bitevents::BitEvent;
use actix::prelude::*;
use std::collections::BTreeMap;
use std::sync::mpsc::Sender;
use std::time::Instant;

pub fn default_handler_event() -> (String, u8) {
    (String::from(crate::DEFAULT_NAME), 0)
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct InputEvents {
    pub time: Instant,
    pub events: Vec<BitEvent>,
}

impl InputEvents {
    pub fn for_events(events: Vec<BitEvent>) -> InputEvents {
        InputEvents {
            time: Instant::now(),
            events,
        }
    }
}

impl Actor for Simulator {
    type Context = Context<Self>;
}

impl Handler<InputEvents> for Simulator {
    type Result = ();

    fn handle(&mut self, events: InputEvents, _ctxt: &mut <Self>::Context) -> Self::Result {
        debug!(
            "Processing events at {}",
            events.time.duration_since(self.start_time).as_secs_f64()
        );
        self.process(&events.events);
    }
}

// Map a device name and bit number to the handler
pub type HandlerMap = BTreeMap<(String, u8), EventHandler>;

pub struct Simulator {
    start_time: Instant,
    handlers: HandlerMap,
    sender: Sender<BitEvent>,
}

impl Simulator {
    pub fn new(handlers: HandlerMap, sender: &Sender<BitEvent>) -> Simulator {
        Simulator {
            start_time: Instant::now(),
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
                .or_else(|| self.handlers.get(&default_handler_event()));

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
