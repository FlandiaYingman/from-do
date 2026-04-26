#[cfg(test)]
mod tests {
    use super::super::pattern::{DayPattern, Item, MonthPattern, Pattern, YearPattern};
    use super::super::{strfrecur, strprecur};

    /// Format every pattern then parse the result back, asserting equality
    /// with the normalized form of the original.
    #[test]
    fn roundtrip_representative_patterns() {
        let patterns = [
            // wildcards
            Pattern::new(
                DayPattern::Wildcard,
                MonthPattern::Wildcard,
                YearPattern::Wildcard,
            ),
            // weekday atoms
            Pattern::new(
                DayPattern::DayOfWeek(vec![Item::Atom(1)]),
                MonthPattern::Wildcard,
                YearPattern::Wildcard,
            ),
            Pattern::new(
                DayPattern::DayOfWeek(vec![Item::Atom(7)]),
                MonthPattern::Wildcard,
                YearPattern::Wildcard,
            ),
            // monthday atoms (positive and negative)
            Pattern::new(
                DayPattern::DayOfMonth(vec![Item::Atom(1)]),
                MonthPattern::Wildcard,
                YearPattern::Wildcard,
            ),
            Pattern::new(
                DayPattern::DayOfMonth(vec![Item::Atom(15)]),
                MonthPattern::Wildcard,
                YearPattern::Wildcard,
            ),
            Pattern::new(
                DayPattern::DayOfMonth(vec![Item::Atom(-1)]),
                MonthPattern::Wildcard,
                YearPattern::Wildcard,
            ),
            Pattern::new(
                DayPattern::DayOfMonth(vec![Item::Atom(-3)]),
                MonthPattern::Wildcard,
                YearPattern::Wildcard,
            ),
            // weekday range
            Pattern::new(
                DayPattern::DayOfWeek(vec![Item::Range(1, 5, 1)]),
                MonthPattern::Wildcard,
                YearPattern::Wildcard,
            ),
            // weekday stepped wildcard range
            Pattern::new(
                DayPattern::DayOfWeek(vec![Item::Range(1, 7, 2)]),
                MonthPattern::Wildcard,
                YearPattern::Wildcard,
            ),
            // monthday stepped wildcard range
            Pattern::new(
                DayPattern::DayOfMonth(vec![Item::Range(1, 31, 2)]),
                MonthPattern::Wildcard,
                YearPattern::Wildcard,
            ),
            // weekday list
            Pattern::new(
                DayPattern::DayOfWeek(vec![Item::Atom(1), Item::Atom(3), Item::Atom(5)]),
                MonthPattern::Wildcard,
                YearPattern::Wildcard,
            ),
            // monthday list with last
            Pattern::new(
                DayPattern::DayOfMonth(vec![Item::Atom(1), Item::Atom(-1)]),
                MonthPattern::Wildcard,
                YearPattern::Wildcard,
            ),
            // month atom
            Pattern::new(
                DayPattern::Wildcard,
                MonthPattern::List(vec![Item::Atom(1)]),
                YearPattern::Wildcard,
            ),
            // month range
            Pattern::new(
                DayPattern::Wildcard,
                MonthPattern::List(vec![Item::Range(1, 6, 1)]),
                YearPattern::Wildcard,
            ),
            // month stepped wildcard
            Pattern::new(
                DayPattern::Wildcard,
                MonthPattern::List(vec![Item::Range(1, 12, 2)]),
                YearPattern::Wildcard,
            ),
            // month stepped range
            Pattern::new(
                DayPattern::Wildcard,
                MonthPattern::List(vec![Item::Range(1, 6, 2)]),
                YearPattern::Wildcard,
            ),
            // year atom
            Pattern::new(
                DayPattern::Wildcard,
                MonthPattern::Wildcard,
                YearPattern::List(vec![Item::Atom(2026)]),
            ),
            // year range
            Pattern::new(
                DayPattern::Wildcard,
                MonthPattern::Wildcard,
                YearPattern::List(vec![Item::Range(2026, 2030, 1)]),
            ),
            // year stepped range
            Pattern::new(
                DayPattern::Wildcard,
                MonthPattern::Wildcard,
                YearPattern::List(vec![Item::Range(2026, 2030, 2)]),
            ),
            // year stepped wildcard
            Pattern::new(
                DayPattern::Wildcard,
                MonthPattern::Wildcard,
                YearPattern::List(vec![Item::Range(1, 9999, 4)]),
            ),
            // combined
            Pattern::new(
                DayPattern::DayOfWeek(vec![Item::Atom(1)]),
                MonthPattern::List(vec![Item::Atom(1)]),
                YearPattern::List(vec![Item::Atom(2026)]),
            ),
            Pattern::new(
                DayPattern::DayOfWeek(vec![Item::Atom(1)]),
                MonthPattern::List(vec![Item::Range(1, 6, 1)]),
                YearPattern::List(vec![Item::Atom(2026)]),
            ),
        ];

        for pattern in patterns {
            let normalized = pattern.normalized();
            let formatted = strfrecur(&pattern);
            let parsed = strprecur(&formatted)
                .unwrap_or_else(|err| panic!("parse failed: {formatted:?}: {err}"));

            assert_eq!(
                parsed, normalized,
                "pattern: {pattern:?}; formatted: {formatted:?}"
            );
        }
    }
}
