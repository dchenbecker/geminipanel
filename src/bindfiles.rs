use crate::input::InputError;
use crate::simulation::{EventHandler, HandlerFunc};

use sdl2::mixer;

type SoundFileSpec = Option<(String, i32)>;

pub fn create_handler(
    handler_name: &'static str,
    on_file_spec: SoundFileSpec,
    off_file_spec: SoundFileSpec,
) -> Result<Option<EventHandler>, InputError> {
    let on_file = create_chunk(on_file_spec)?;
    let off_file = create_chunk(off_file_spec)?;

    if on_file.is_some() || off_file.is_some() {
        let handler_func: HandlerFunc = Box::new(move |value, _| {
            if value == 0 {
                if let Some(off_chunk) = off_file {
                    info!("Playing off sound for {}", handler_name);
                    if let Err(e) = mixer::Channel::all().play(&off_chunk, 0) {
                        warn!("Error playing {}: {}", handler_name, e);
                    };
                }
            }

            if value == 1 {
                if let Some(on_chunk) = on_file {
                    info!("Playing on sound for {}", handler_name);
                    if let Err(e) = mixer::Channel::all().play(&on_chunk, 0) {
                        warn!("Error playing {}: {}", handler_name, e);
                    };
                }
            }
        });

        Ok(Some(EventHandler::new(handler_name, handler_func)))
    } else {
        Ok(None)
    }
}

fn create_chunk(spec: SoundFileSpec) -> Result<Option<mixer::Chunk>, InputError> {
    spec.map(|(filename, volume)| {
        let mut chunk = mixer::Chunk::from_file(filename).map_err(InputError::new)?;
        chunk.set_volume(volume);
        Ok(chunk)
    })
    .transpose()
}

// Perform split and basic validation of the line
pub fn split_sound_line(line: &str) -> Result<Vec<&str>, InputError> {
    let parts: Vec<&str> = line.split(',').collect();

    debug!("Got definition for input {:?}", parts);

    if parts.len() != 5 {
        return Err(InputError::new(format!(
            "Incorrect number of elements for {}",
            line
        )));
    }

    if parts[2].trim().is_empty() {
        return Err(InputError::new(format!("Missing name for event: {}", line)));
    }

    Ok(parts)
}

// Parse the filename to determine whether there's a sound, and if so, the optional volume, if specified. Default to
// music::MAX_VOLUME
pub fn parse_sound_filename(filename: &str) -> Result<SoundFileSpec, InputError> {
    use std::str::FromStr;

    if filename.is_empty() {
        Ok(None)
    } else {
        let parts: Vec<_> = filename.split(':').collect();

        match parts.len() {
            1 => Ok(Some((parts[0].to_string(), mixer::MAX_VOLUME))),
            2 => {
                let volume = (f64::from_str(parts[1])? * (mixer::MAX_VOLUME as f64)) as i32;

                Ok(Some((parts[0].to_string(), volume)))
            }
            invalid => Err(InputError::new(format!(
                "Invalid sound file spec '{}': {} parts",
                filename, invalid
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::parse_sound_filename;
    use sdl2::mixer;

    #[test]
    fn test_empty_filename() {
        assert!(parse_sound_filename("") == Ok(None));
    }

    #[test]
    fn test_filename_only() {
        assert!(
            parse_sound_filename("testing") == Ok(Some(("testing".to_string(), mixer::MAX_VOLUME)))
        );
    }

    #[test]
    fn test_filename_with_volume() {
        assert!(parse_sound_filename("testing:0.5") == Ok(Some(("testing".to_string(), 64))));
    }

    #[test]
    fn test_filename_too_many_components() {
        assert!(parse_sound_filename("testing:0.5:oops").is_err());
    }

    #[test]
    fn test_filename_bad_volume() {
        assert!(parse_sound_filename("testing:goes to 11").is_err());
    }
}
