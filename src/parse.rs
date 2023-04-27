use std::fmt;

#[derive(Debug, Clone)]
pub enum ParsingError {
    NotANumber(f64),
    Failed(String),
    Missing,
}

impl fmt::Display for ParsingError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use ParsingError::*;
        match self {
            NotANumber(value) => write!(f, "{} is not a number", value),
            Failed(line) => {
                write!(f, "parsing {} failed", line)
            }
            Missing => write!(f, "nothing to read"),
        }
    }
}

impl PartialEq for ParsingError {
    fn eq(&self, other: &Self) -> bool {
        use ParsingError::*;
        matches!(
            (self, other),
            (NotANumber(_), NotANumber(_)) | (Missing, Missing) | (Failed(_), Failed(_))
        )
    }
}

/// Parse the value at `index` position as a double.
///
/// # Arguments
/// * `line` - String input to be parser
/// * `index` - Index of the field in `line`, where the fields are whitespace separated
///
/// # Errors
///
/// It will throw error in two cases:
/// * It was not able to parse the string as a `f64` number.
/// * The parsed value is `f64::NAN` or infinite.
pub fn parse(line: String, index: usize) -> Result<f64, ParsingError> {
    if let Some(field) = line.split_whitespace().nth(index) {
        match field.parse::<f64>() {
            Ok(value) => {
                if value.is_nan() || value.is_infinite() {
                    return Err(ParsingError::NotANumber(value));
                }
                Ok(value)
            }
            Err(_) => Err(ParsingError::Failed(field.to_owned())),
        }
    } else {
        Err(ParsingError::Missing)
    }
}

#[cfg(test)]
mod tests {
    use super::{parse, ParsingError};

    #[test]
    fn parse_ok() {
        assert_eq!(parse(String::from("0.00001"), 0), Ok(0.00001));
        assert_eq!(parse(String::from("3.14 25.13 31 42"), 0), Ok(3.14));
        assert_eq!(parse(String::from("3.14 25.13 31 42"), 3), Ok(42.0));
    }

    #[test]
    fn parse_err() {
        assert_eq!(parse(String::from(""), 0), Err(ParsingError::Missing));
        assert_eq!(parse(String::from(""), 5), Err(ParsingError::Missing));
        assert_eq!(parse(String::from("1 2 3"), 5), Err(ParsingError::Missing));
        assert_eq!(
            parse(String::from("NaN"), 0),
            Err(ParsingError::NotANumber(f64::NAN))
        );
        assert_eq!(
            parse(String::from("inf"), 0),
            Err(ParsingError::NotANumber(f64::INFINITY))
        );
        assert_eq!(
            parse(String::from("1 2 3efg7"), 2),
            Err(ParsingError::Failed(String::from("3efg7")))
        );
    }
}
