use super::{error::Error, token::Token};

type Operation = fn(f64, f64) -> f64;

/// Binding powers, higher binding tighter. Infix operators carry a left and a
/// right power; a right associative operator has the lower one on the right.
///
/// Prefix sub sits below the exponent's left power so -2^2 is -(2^2), and
/// postfix percent sits above everything so 15% is a value before `of` sees it.
const NEGATE: u8 = 5;
const PERCENT: u8 = 9;

struct Parser<'a> {
    tokens: &'a [Token],
    index: usize,
}

impl Parser<'_> {
    fn peek(&self) -> Option<Token> {
        self.tokens.get(self.index).copied()
    }

    fn advance(&mut self) -> Option<Token> {
        let token = self.peek()?;
        self.index += 1;
        Some(token)
    }

    fn expression(&mut self, min: u8) -> Result<f64, Error> {
        let mut left = self.prefix()?;

        while let Some(token) = self.peek() {
            if token == Token::Percent {
                if PERCENT < min {
                    break;
                }

                self.index += 1;
                left /= 100.0;
                continue;
            }

            let Some((left_power, right_power, operation)) = infix(token) else {
                break;
            };

            if left_power < min {
                break;
            }

            self.index += 1;
            let right = self.expression(right_power)?;
            left = operation(left, right);
        }

        Ok(left)
    }

    fn prefix(&mut self) -> Result<f64, Error> {
        match self.advance().ok_or(Error::UnexpectedEnd)? {
            Token::Number(value) => Ok(value),
            Token::Sub => Ok(-self.expression(NEGATE)?),
            Token::Add => self.expression(NEGATE),
            Token::Open => {
                let value = self.expression(0)?;

                match self.advance() {
                    Some(Token::Close) => Ok(value),
                    _ => Err(Error::Unclosed),
                }
            }
            _ => Err(Error::UnexpectedToken),
        }
    }
}

fn infix(token: Token) -> Option<(u8, u8, Operation)> {
    match token {
        Token::Add => Some((1, 2, |left, right| left + right)),
        Token::Sub => Some((1, 2, |left, right| left - right)),
        Token::Mul | Token::Of => Some((3, 4, |left, right| left * right)),
        Token::Div => Some((3, 4, |left, right| left / right)),
        Token::Mod => Some((3, 4, |left, right| left % right)),
        Token::Pow => Some((6, 5, f64::powf)),
        _ => None,
    }
}

pub fn evaluate(tokens: &[Token]) -> Result<f64, Error> {
    let mut parser = Parser { tokens, index: 0 };
    let value = parser.expression(0)?;

    if parser.peek().is_some() {
        return Err(Error::UnexpectedToken);
    }

    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::evaluate;
    use crate::calc::{error::Error, token::tokenize};

    fn value(input: &str) -> f64 {
        evaluate(&tokenize(input).expect("tokenizes")).expect("evaluates")
    }

    fn error(input: &str) -> Error {
        evaluate(&tokenize(input).expect("tokenizes")).expect_err("rejects")
    }

    #[test]
    fn multiplication_binds_tighter_than_addition() {
        assert_eq!(value("2+3*4"), 14.0);
        assert_eq!(value("(2+3)*4"), 20.0);
    }

    #[test]
    fn exponentiation_is_right_associative() {
        assert_eq!(value("2^3^2"), 512.0);
    }

    #[test]
    fn prefix_minus_binds_looser_than_exponentiation() {
        assert_eq!(value("-2^2"), -4.0);
        assert_eq!(value("(-2)^2"), 4.0);
    }

    #[test]
    fn percent_is_postfix_and_of_multiplies() {
        assert_eq!(value("15%"), 0.15);
        assert_eq!(value("15%of240"), 36.0);
        assert_eq!(value("15% of 240"), 36.0);
    }

    #[test]
    fn percent_of_composes_with_arithmetic() {
        assert_eq!(value("10+15%of240"), 46.0);
    }

    #[test]
    fn mod_is_modulo() {
        assert_eq!(value("10mod3"), 1.0);
        assert_eq!(value("10 mod 3"), 1.0);
    }

    #[test]
    fn spaces_are_never_required() {
        assert_eq!(value("36*3"), 108.0);
        assert_eq!(value("(3+4)^2/7"), 7.0);
    }

    #[test]
    fn incomplete_expressions_are_rejected() {
        assert_eq!(error("2+"), Error::UnexpectedEnd);
        assert_eq!(error("(2"), Error::Unclosed);
        assert_eq!(error("2 3"), Error::UnexpectedToken);
        assert_eq!(error("*2"), Error::UnexpectedToken);
    }
}
