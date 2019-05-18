use input::InputError;
use simulation::{EventHandler, HandlerFunc};
use to_static;

type SoundFile = Option<(&'static String, f64)>;

pub fn create_handler(
    handler_name: &'static str,
    on_file: SoundFile,
    off_file: SoundFile,
) -> Option<EventHandler> {
    if on_file.is_some() || off_file.is_some() {
        let handler_func: HandlerFunc = Box::new(move |value, _| {
            if value == 0 {
                if let Some((off_filename, volume)) = off_file {
                    info!("Playing off sound for {}", handler_name);
                    music::play_sound(&off_filename, music::Repeat::Times(0), volume);
                }
            }

            if value == 1 {
                if let Some((on_filename, volume)) = on_file {
                    info!("Playing on sound for {}", handler_name);
                    music::play_sound(&on_filename, music::Repeat::Times(0), volume);
                }
            }
        });

        Some(EventHandler::new(handler_name, handler_func))
    } else {
        None
    }
}

// Perform split and basic validation of the line
pub fn split_sound_line(line: &str) -> Result<Vec<&str>, InputError> {
    let parts: Vec<&str> = line.split(",").collect();

    debug!("Got definition for input {:?}", parts);

    if parts.len() != 4 {
        return Err(InputError::new(format!(
            "Incorrect number of elements for {}",
            line
        )));
    }

    if parts[1].trim().is_empty() {
        return Err(InputError::new(format!("Missing name for event: {}", line)));
    }

    Ok(parts)
}

// Parse the filename to determine whether there's a sound, and if so, the optional volume, if specified. Default to
// music::MAX_VOLUME
pub fn parse_sound_filename(filename: &str) -> Result<SoundFile, InputError> {
    use std::str::FromStr;

    if filename.is_empty() {
        Ok(None)
    } else {
        let parts: Vec<_> = filename.split(":").collect();

        match parts.len() {
            1 => Ok(Some((to_static(parts[0]), music::MAX_VOLUME))),
            2 => {
                let volume = f64::from_str(parts[1])?;
                Ok(Some((to_static(parts[0]), volume)))
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

    #[test]
    fn test_empty_filename() {
        assert!(parse_sound_filename("") == Ok(None));
    }

    #[test]
    fn test_filename_only() {
        assert!(
            parse_sound_filename("testing")
                == Ok(Some((&"testing".to_string(), music::MAX_VOLUME)))
        );
    }

    #[test]
    fn test_filename_with_volume() {
        assert!(parse_sound_filename("testing:0.5") == Ok(Some((&"testing".to_string(), 0.5))));
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
