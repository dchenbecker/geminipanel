use crate::input::bitevents::BitEvent;
use actix::prelude::*;
use std::collections::BTreeMap;
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

    fn handle(&mut self, events: InputEvents, ctxt: &mut <Self>::Context) -> Self::Result {
        debug!(
            "Processing events at {}",
            events.time.duration_since(self.start_time).as_secs_f64()
        );
        self.process(&events.events, &ctxt.address().recipient());
    }
}

// This is temprary until we can build an outpu actor
#[derive(Debug, Message)]
#[rtype(result = "()")]
pub struct OutputEvent {
    pub time: Instant,
    pub events: Vec<BitEvent>,
}

impl OutputEvent {
    fn for_events(events: Vec<BitEvent>) -> OutputEvent {
        OutputEvent {
            time: Instant::now(),
            events,
        }
    }
}

impl Handler<OutputEvent> for Simulator {
    type Result = ();

    fn handle(&mut self, events: OutputEvent, _ctxt: &mut <Self>::Context) -> Self::Result {
        // Noop for now
        println!("Recevied output: {:?}", events);
    }
}

// Map a device name and bit number to the handler
pub type HandlerMap = BTreeMap<(String, u8), EventHandler>;

pub struct Simulator {
    start_time: Instant,
    handlers: HandlerMap,
}

impl Simulator {
    pub fn new(handlers: HandlerMap) -> Simulator {
        Simulator {
            start_time: Instant::now(),
            handlers,
        }
    }

    pub fn process(&self, events: &[BitEvent], recipient: &Recipient<OutputEvent>) {
        debug!("Processing {} simulation input events", events.len());
        for event in events {
            let target_handler = self
                .handlers
                .get(&(event.dev_name.clone(), event.bit))
                .or_else(|| self.handlers.get(&default_handler_event()));

            if let Some(to_fire) = target_handler {
                info!("Firing '{}' for event {:?}", to_fire.name, event);
                (to_fire.handler)(event.value, recipient);
            } else {
                warn!("Event without a handler: {}", event);
            }
        }
    }
}

pub type HandlerFunc = Box<dyn Fn(u8, &Recipient<OutputEvent>) -> ()>;

pub struct EventHandler {
    name: &'static str,
    handler: HandlerFunc,
}

impl EventHandler {
    pub fn new(name: &'static str, handler: HandlerFunc) -> EventHandler {
        EventHandler { name, handler }
    }
}
