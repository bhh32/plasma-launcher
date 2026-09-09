#[derive(Debug, Clone, Default)]
pub struct Trigger {
    pub prefixes: Vec<String>,
    pub starts_with: Vec<char>,
    pub digits: bool,
    pub isolate: Vec<String>,
    pub always: bool,
}

impl Trigger {
    pub fn accepts(&self, query: &str) -> bool {
        if self.always {
            return true;
        }
        let query = query.trim_start();
        if query.is_empty() {
            return false;
        }
        self.matches_prefix(query) || self.matches_start(query)
    }

    pub fn isolates(&self, query: &str) -> bool {
        let query = query.trim_start();
        self.isolate.iter().any(|token| query.starts_with(token))
    }

    // A prefix is routing, not content, so the plugin is sent what follows
    // it. Anything matched by starts_with, digits or always is content and
    // arrives whole.
    pub fn strip<'a>(&self, query: &'a str) -> &'a str {
        let trimmed = query.trim_start();
        let Some((first, rest)) = trimmed.split_once(char::is_whitespace) else {
            return query;
        };
        if self.prefixes.iter().any(|prefix| prefix == first) {
            rest.trim_start()
        } else {
            query
        }
    }

    fn matches_prefix(&self, query: &str) -> bool {
        let Some((first, rest)) = query.split_once(char::is_whitespace) else {
            return false;
        };
        !rest.trim().is_empty() && self.prefixes.iter().any(|prefix| prefix == first)
    }

    fn matches_start(&self, query: &str) -> bool {
        let Some(first) = query.chars().next() else {
            return false;
        };
        self.starts_with.contains(&first) || (self.digits && first.is_ascii_digit())
    }
}

#[cfg(test)]
mod tests {
    use super::Trigger;

    fn calculator() -> Trigger {
        Trigger {
            starts_with: vec!['=', '(', '-', '.'],
            digits: true,
            isolate: vec!["=".to_owned()],
            ..Trigger::default()
        }
    }

    fn find() -> Trigger {
        Trigger {
            prefixes: vec!["find".to_owned()],
            isolate: vec!["find".to_owned()],
            ..Trigger::default()
        }
    }

    #[test]
    fn a_leading_char_need_no_space() {
        let trigger = calculator();

        assert!(trigger.accepts("=2+2"));
        assert!(trigger.accepts("36*3"));
        assert!(!trigger.accepts("firefox"));
    }

    #[test]
    fn a_prefix_needs_a_space_and_something_after_it() {
        let trigger = find();

        assert!(trigger.accepts("find report"));
        assert!(!trigger.accepts("find"));
        assert!(!trigger.accepts("find  "));
        assert!(!trigger.accepts("finder report"));
    }

    #[test]
    fn isolation_is_independent_of_acceptance() {
        let trigger = calculator();

        assert!(trigger.accepts("36*3"));
        assert!(!trigger.isolates("36*3"));
        assert!(trigger.isolates("= 36*3"));
    }

    #[test]
    fn always_answers_everything_except_nothing() {
        let trigger = Trigger {
            always: true,
            ..Trigger::default()
        };

        assert!(trigger.accepts("anything"));
        assert!(trigger.accepts(""));
    }

    #[test]
    fn an_empty_trigger_answers_nothing() {
        let trigger = Trigger::default();

        assert!(!trigger.accepts("anything"));
        assert!(!trigger.isolates("anything"));
    }

    #[test]
    fn a_matched_prefix_is_stripped() {
        assert_eq!(find().strip("find report.odt"), "report.odt");
        assert_eq!(find().strip("find  a b"), "a b");
    }

    #[test]
    fn content_arrives_whole() {
        // The calculator matches on a leading character, so nothing is routing
        assert_eq!(calculator().strip("36 * 3"), "36 * 3");
        assert_eq!(calculator().strip("= 2+2"), "= 2+2");
    }

    #[test]
    fn an_unmatched_first_word_is_left_alone() {
        assert_eq!(find().strip("finder report"), "finder report");
    }
}
