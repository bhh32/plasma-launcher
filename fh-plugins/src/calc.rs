pub mod error;
pub mod parser;
pub mod token;

use fh_ipc::{IconSource, Indice, PluginResponse, PluginSearchResult};
use fh_service::{Plugin, Usage};

use crate::calc::{parser::evaluate, token::tokenize};

#[derive(Default)]
pub struct Calculator {
    outcome: Option<String>,
}

impl Plugin for Calculator {
    fn name(&self) -> &'static str {
        "calculator"
    }

    fn accepts(&self, query: &str) -> bool {
        query
            .trim_start()
            .starts_with(|c: char| c.is_ascii_digit() || matches!(c, '=' | '(' | '-' | '.'))
    }

    fn isolates(&self, query: &str) -> bool {
        query.trim_start().starts_with('=')
    }

    fn search(&mut self, query: &str) -> Vec<PluginSearchResult> {
        self.outcome = None;

        let query = query.trim();
        let (expression, explicit) = match query.strip_prefix('=') {
            Some(rest) => (rest.trim(), true),
            None => (query, false),
        };
        let Ok(tokens) = tokenize(expression) else {
            return Vec::new();
        };

        // A lone number is not a calculation, so typing "5" offers
        // apps rather than an answer of five. "= 5" always answers.
        if !explicit && tokens.len() < 2 {
            return Vec::new();
        }
        let Ok(value) = evaluate(&tokens) else {
            return Vec::new();
        };

        // Division by zero and the like reach here as infinity or NaN.
        if !value.is_finite() {
            return Vec::new();
        }

        let formatted = format_value(value);
        self.outcome = Some(formatted.clone());

        vec![PluginSearchResult {
            id: 0,
            name: formatted,
            description: expression.to_owned(),
            keywords: None,
            icon: Some(IconSource::Name("accessories-calculator".to_owned())),
            exec: None,
            window: None,
        }]
    }

    fn activate(&mut self, _id: Indice) -> Vec<PluginResponse> {
        // Fill without Close: the answer replaces the expression and the
        // launcher stays open so it can be operated on again.
        match self.outcome.as_deref() {
            Some(value) => vec![PluginResponse::Fill(format!("= {value}"))],
            None => Vec::new(),
        }
    }

    fn usage(&self) -> Vec<Usage> {
        vec![Usage {
            prefix: "=".into(),
            example: "= 15%of240".into(),
            description: "Calculator".into(),
        }]
    }
}

fn format_value(value: f64) -> String {
    // Binary floating point leaves 0.1 + 0.2 at 0.3000...4. Rounding
    // to ten decimals hides that without hiding digits needed.
    if value.abs() < 1e15 {
        let rounded = (value * 1e10).round() / 1e10;
        format!("{rounded}")
    } else {
        format!("{value}")
    }
}

#[cfg(test)]
mod tests {
    use super::Calculator;
    use fh_ipc::PluginResponse;
    use fh_service::Plugin;

    #[test]
    fn a_lone_number_is_not_a_calc() {
        let mut calc = Calculator::default();

        assert!(calc.search("5").is_empty());
        assert_eq!(calc.search("= 5").len(), 1);
    }

    #[test]
    fn the_answer_is_the_result_name() {
        let mut calc = Calculator::default();
        let results = calc.search("15%of240");

        assert_eq!(results[0].name, "36");
        assert_eq!(results[0].description, "15%of240");
    }

    #[test]
    fn floating_point_noise_is_rounded_away() {
        let mut calc = Calculator::default();
        assert_eq!(calc.search("0.1+0.2")[0].name, "0.3");
    }
    #[test]
    fn nonsense_produces_no_result() {
        let mut calc = Calculator::default();

        assert!(calc.search("2+").is_empty());
        assert!(calc.search("1/0").is_empty());
    }

    #[test]
    fn activating_fills_without_close() {
        let mut calc = Calculator::default();
        calc.search("36*3");

        assert_eq!(calc.activate(0), vec![PluginResponse::Fill("= 108".into())]);
    }

    #[test]
    fn only_an_equals_prefix_isolates() {
        let calc = Calculator::default();

        assert!(calc.isolates("= 2+2"));
        assert!(!calc.isolates("2+2"));
        assert!(calc.accepts("2+2"));
        assert!(!calc.accepts("firefox"));
    }
}
