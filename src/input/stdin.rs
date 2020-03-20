use super::InputError;
use crate::input::bitevents::BitEvent;
use crate::simulation::InputEvents;

use actix::prelude::*;

use tokio::io;
use tokio::io::AsyncBufReadExt;
use tokio::stream::StreamExt;

pub struct StdinInput {
    pub recipient: Recipient<InputEvents>,
}

impl Actor for StdinInput {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Context<Self>) {
        // Create a stream to read
        let stdin = io::stdin();
        let reader = io::BufReader::new(stdin);

        Self::add_stream(reader.lines().map(|l| l.unwrap()), ctx);
    }
}

impl StreamHandler<String> for StdinInput {
    fn handle(&mut self, input: String, _: &mut Self::Context) {
        match parse_events(&input) {
            Ok(events) => self
                .recipient
                .do_send(InputEvents::for_events(events))
                .unwrap(),
            Err(e) => warn!("{}", e),
        }
    }
}

fn parse_events(input: &str) -> Result<Vec<BitEvent>, InputError> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_valid_pattern() {
        assert_eq!(
            Ok(vec!(
                BitEvent {
                    dev_name: "test".to_string(),
                    bit: 1,
                    value: 1
                },
                BitEvent {
                    dev_name: "foo".to_string(),
                    bit: 42,
                    value: 0
                }
            )),
            parse_events("test:1:1,foo:42:0")
        );
    }

    #[test]
    fn test_parse_invalid_pattern() {
        assert!(parse_events("not a valid pattern").is_err());
    }

    #[test]
    fn test_parse_invalid_bit() {
        assert!(parse_events("test:test:0").is_err());
    }

    #[test]
    fn test_parse_invalid_value() {
        assert!(parse_events("test:12:off").is_err());
    }
}
