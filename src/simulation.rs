use input::bitevents::BitEvent;
use std::sync::mpsc::Sender;

pub struct Simulator {
    handlers: Vec<EventHandler>,
    sender: Sender<BitEvent>,
}

impl Simulator {
    pub fn new(handlers: Vec<EventHandler>, sender: &Sender<BitEvent>) -> Simulator {
        Simulator { handlers: handlers, sender: (*sender).clone() }
    }

    pub fn process(&self, events: &[BitEvent]) {
        for event in events {
            if event.bit > self.handlers.len() {
                warn!("Event without a handler on index {}", event.bit);
            } else {
                let to_fire = &self.handlers[event.bit];
                info!("Firing {} for event {:?}", to_fire.name, event);
                (to_fire.handler)(event.value, &self.sender);
            }
        }
    }
}

type HandlerFunc = fn(u8, &Sender<BitEvent>) -> ();

pub struct EventHandler {
    name: &'static str,
    handler: HandlerFunc,
}

impl EventHandler {
    pub fn new(name: &'static str, handler: HandlerFunc) -> EventHandler {
        EventHandler { name, handler }
    }
}
