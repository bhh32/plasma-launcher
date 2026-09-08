use super::error::Error;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Token {
    Number(f64),
    Add,
    Sub,
    Mul,
    Div,
    Pow,
    Percent,
    Mod,
    Of,
    Open,
    Close,
}

pub fn tokenize(input: &str) -> Result<Vec<Token>, Error> {
    let chars: Vec<char> = input.chars().collect();
    let mut tokens = Vec::new();
    let mut idx = 0;

    while idx < chars.len() {
        let ch = chars[idx];

        // Dropping whitespace here allows "15%of240" and
        // "15 % of 240" to be the same expression
        if ch.is_whitespace() {
            idx += 1;
            continue;
        }

        if ch.is_ascii_digit() || ch == '.' {
            let start = idx;

            while idx < chars.len() && (chars[idx].is_ascii_digit() || chars[idx] == '.') {
                idx += 1;
            }

            let text: String = chars[start..idx].iter().collect();
            let value = text
                .parse()
                .map_err(|_| Error::InvalidNumber(text.clone()))?;

            tokens.push(Token::Number(value));
            continue;
        }

        if ch.is_alphabetic() {
            let start = idx;

            while idx < chars.len() && chars[idx].is_alphabetic() {
                idx += 1;
            }

            let word: String = chars[start..idx].iter().collect();

            tokens.push(match word.to_ascii_lowercase().as_str() {
                "mod" => Token::Mod,
                "of" => Token::Of,
                _ => return Err(Error::UnknownWord(word)),
            });

            continue;
        }

        tokens.push(match ch {
            '+' => Token::Add,
            '-' => Token::Sub,
            '*' => Token::Mul,
            '/' => Token::Div,
            '^' => Token::Pow,
            '%' => Token::Percent,
            '(' => Token::Open,
            ')' => Token::Close,
            _ => return Err(Error::UnexpectedChar(ch)),
        });

        idx += 1;
    }

    Ok(tokens)
}

#[cfg(test)]
mod tests {
    use super::super::error::Error;
    use super::{Token, tokenize};

    #[test]
    fn spacing_does_not_change_the_token_stream() {
        let expected = vec![
            Token::Number(15.0),
            Token::Percent,
            Token::Of,
            Token::Number(240.0),
        ];

        assert_eq!(tokenize("15%of240").expect("tokenizes"), expected);
        assert_eq!(tokenize("15% of 240").expect("tokenizes"), expected);
        assert_eq!(tokenize("  15  %  of  240").expect("tokenizes"), expected);
    }

    #[test]
    fn words_are_case_insensitive() {
        assert_eq!(
            tokenize("10MOD3").expect("tokenizes"),
            vec![Token::Number(10.0), Token::Mod, Token::Number(3.0)]
        );
    }

    #[test]
    fn decimals_parse() {
        assert_eq!(
            tokenize("3.5+.5").expect("tokenizes"),
            vec![Token::Number(3.5), Token::Add, Token::Number(0.5)]
        );
    }

    #[test]
    fn unknown_words_and_characters_are_rejected() {
        assert_eq!(tokenize("2 foo 3"), Err(Error::UnknownWord("foo".into())));
        assert_eq!(tokenize("2 $ 3"), Err(Error::UnexpectedChar('$')));
    }
}
