use actix::prelude::*;

use crate::input::bitevents::BitEvent;
use std::time::Duration;

pub struct Blinker {
    event: BitEvent,
    recipient: Recipient<BitEvent>,
    count: u8,
    interval: Duration,
}

impl Blinker {
    pub fn on_interval(
        event: BitEvent,
        recipient: Recipient<BitEvent>,
        count: u8,
        interval: Duration,
    ) -> Addr<Blinker> {
        Blinker {
            event,
            recipient,
            count,
            interval,
        }
        .start()
    }
}

impl Actor for Blinker {
    type Context = Context<Self>;

    fn started(&mut self, ctxt: &mut Self::Context) {
        ctxt.run_interval(self.interval, |act, context| {
            if act.count == 0 {
                // Cancel, we're done
                context.stop();
            } else {
                let to_send = BitEvent {
                    dev_name: act.event.dev_name.clone(),
                    value: act.count % 2,
                    ..act.event
                };

                act.recipient.do_send(to_send).unwrap();
            }
        });
    }
}
