use crate::error::Error;
use serde::{Serialize, de::DeserializeOwned};

pub fn encode_line<T: Serialize>(message: &T) -> Result<String, Error> {
    let mut line = serde_json::to_string(message)?;
    line.push('\n');
    Ok(line)
}

pub fn decode_line<T: DeserializeOwned>(line: &str) -> Result<T, Error> {
    let trimmed = line.trim();

    if trimmed.is_empty() {
        return Err(Error::Empty);
    }

    Ok(serde_json::from_str(trimmed)?)
}

#[cfg(test)]
mod tests {
    use super::{decode_line, encode_line};
    use crate::{Request, error::Error};

    #[test]
    fn a_query_containing_a_newline_cannot_split_the_frame() {
        let line = encode_line(&Request::Search("a\nb".into())).expect("encodes");

        assert_eq!(line, "{\"Search\":\"a\\nb\"}\n");
        assert_eq!(line.matches('\n').count(), 1);
    }

    #[test]
    fn blank_lines_are_empty_not_malformed() {
        assert!(matches!(decode_line::<Request>("  \n"), Err(Error::Empty)));
    }

    #[test]
    fn junk_is_malformed() {
        assert!(matches!(
            decode_line::<Request>("not json"),
            Err(Error::Malformed(_))
        ));
    }
}
