use input::InputError;
use to_static;

// Parse the filename to determine whether there's a sound, and if so, the optional volume, if specified. Default to
// music::MAX_VOLUME
pub fn parse_sound_filename(filename: &str) -> Result<Option<(&'static String, f64)>, InputError> {
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
