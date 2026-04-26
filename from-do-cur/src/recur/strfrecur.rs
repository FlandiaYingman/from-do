use super::pattern::{DayPattern, FieldUnit, Item, MonthPattern, Pattern, YearPattern};

pub fn strfrecur(pattern: &Pattern) -> String {
    pattern.normalized().format()
}

impl Pattern {
    fn format(&self) -> String {
        let mut builder = String::new();

        let d = self.d.format();
        builder += &d;

        let m = self.m.format();
        match &self.m {
            MonthPattern::Wildcard => (),
            MonthPattern::List(items) => match items.as_slice() {
                [] => panic!("items.len() == 0"),
                [Item::Atom(_), ..] => {
                    builder += " in ";
                    builder += &m.strip_prefix("every ").unwrap_or(&m);
                }
                [Item::Range(_, _, _), ..] => {
                    builder += " in ";
                    builder += m
                        .strip_prefix("every month in ")
                        .or_else(|| m.strip_prefix("every month "))
                        .unwrap_or(&m);
                }
            },
        };

        let y = self.y.format();
        match &self.y {
            YearPattern::Wildcard => (),
            YearPattern::List(items) => match items.as_slice() {
                [] => panic!("items.len() == 0"),
                [Item::Atom(_), ..] => {
                    builder += " in ";
                    builder += &y.strip_prefix("every ").unwrap_or(&y);
                }
                [Item::Range(_, _, _), ..] => {
                    builder += " in ";
                    builder += y
                        .strip_prefix("every year in ")
                        .or_else(|| y.strip_prefix("every year "))
                        .unwrap_or(&y);
                }
            },
        };

        builder
    }
}

impl DayPattern {
    fn format(&self) -> String {
        match self {
            DayPattern::Wildcard => "every day".to_string(),
            DayPattern::DayOfWeek(items) => format_list(items, FieldUnit::DayOfWeek, "day"),
            DayPattern::DayOfMonth(items) => format_list(items, FieldUnit::DayOfMonth, "day"),
        }
    }
}

impl MonthPattern {
    fn format(&self) -> String {
        match self {
            MonthPattern::Wildcard => "every month".to_string(),
            MonthPattern::List(items) => format_list(items, FieldUnit::Month, "month"),
        }
    }
}

impl YearPattern {
    fn format(&self) -> String {
        match self {
            YearPattern::Wildcard => "every year".to_string(),
            YearPattern::List(items) => format_list(items, FieldUnit::Year, "year"),
        }
    }
}

fn format_list(items: &[Item], unit: FieldUnit, noun: &str) -> String {
    let parts: Vec<String> = items
        .iter()
        .map(|item| format_item(item, unit, noun))
        .collect();
    let body = join_every(&parts);
    format!("every {body}")
}

fn format_item(item: &Item, unit: FieldUnit, noun: &str) -> String {
    match item {
        Item::Atom(v) => format_atom(*v, unit),
        Item::Range(lo, hi, step) => {
            if *step == 1 {
                format!(
                    "{noun} in {}-{}",
                    format_atom(*lo, unit),
                    format_atom(*hi, unit)
                )
            } else if is_full_range((*lo, *hi), unit) {
                format!("{} {noun}", format_ordinal(*step),)
            } else {
                format!(
                    "{} {noun} in {}-{}",
                    format_ordinal(*step),
                    format_atom(*lo, unit),
                    format_atom(*hi, unit)
                )
            }
        }
    }
}

fn is_full_range(range: (i16, i16), unit: FieldUnit) -> bool {
    let (lo, hi) = range;
    if (lo == 0 || lo == 1) && hi == -1 {
        return true;
    }
    if unit == FieldUnit::Month && (lo == 0 || lo == 1) && hi >= 12 {
        return true;
    }
    if unit == FieldUnit::Year && (lo == 0 || lo == 1) && hi >= 9999 {
        return true;
    }
    false
}

fn join_every(parts: &[String]) -> String {
    match parts {
        [] => panic!("parts.len() == 0"),
        [item] => format!("{item}"),
        [a, b] => format!("{a} and {b}"),
        items => {
            let body = items[..items.len() - 1].join(", ");
            format!("{body}, and {}", parts.last().unwrap())
        }
    }
}

fn format_atom(value: i16, unit: FieldUnit) -> String {
    match unit {
        FieldUnit::DayOfWeek => match value {
            1 => "Mon",
            2 => "Tue",
            3 => "Wed",
            4 => "Thu",
            5 => "Fri",
            6 => "Sat",
            7 => "Sun",
            _ => panic!("invalid weekday value: {value}"),
        }
        .to_string(),
        FieldUnit::DayOfMonth => format_ordinal(value),
        FieldUnit::Month => match value {
            1 => "Jan",
            2 => "Feb",
            3 => "Mar",
            4 => "Apr",
            5 => "May",
            6 => "Jun",
            7 => "Jul",
            8 => "Aug",
            9 => "Sep",
            10 => "Oct",
            11 => "Nov",
            12 => "Dec",
            _ => panic!("invalid month value: {value}"),
        }
        .to_string(),
        FieldUnit::Year => value.to_string(),
    }
}

fn format_ordinal(n: i16) -> String {
    match n {
        -1 => "last".to_string(),
        n if n < 0 => format!("{} last", format_ordinal(-n)),
        n => {
            let abs = n.unsigned_abs();
            let suffix = match abs % 100 {
                11 | 12 | 13 => "th",
                _ => match abs % 10 {
                    1 => "st",
                    2 => "nd",
                    3 => "rd",
                    _ => "th",
                },
            };
            format!("{n}{suffix}")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pat(d: DayPattern, m: MonthPattern, y: YearPattern) -> Pattern {
        Pattern::new(d, m, y)
    }

    #[test]
    fn format_day_fragment_canonical() {
        let cases = [
            (
                pat(
                    DayPattern::Wildcard,
                    MonthPattern::Wildcard,
                    YearPattern::Wildcard,
                ),
                "every day",
            ),
            (
                pat(
                    DayPattern::DayOfWeek(vec![Item::Atom(1)]),
                    MonthPattern::Wildcard,
                    YearPattern::Wildcard,
                ),
                "every Mon",
            ),
            (
                pat(
                    DayPattern::DayOfMonth(vec![Item::Atom(1)]),
                    MonthPattern::Wildcard,
                    YearPattern::Wildcard,
                ),
                "every 1st",
            ),
            (
                pat(
                    DayPattern::DayOfMonth(vec![Item::Atom(-1)]),
                    MonthPattern::Wildcard,
                    YearPattern::Wildcard,
                ),
                "every last",
            ),
            (
                pat(
                    DayPattern::DayOfMonth(vec![Item::Atom(-2)]),
                    MonthPattern::Wildcard,
                    YearPattern::Wildcard,
                ),
                "every 2nd last",
            ),
            (
                pat(
                    DayPattern::DayOfWeek(vec![Item::Range(1, 5, 1)]),
                    MonthPattern::Wildcard,
                    YearPattern::Wildcard,
                ),
                "every day in Mon-Fri",
            ),
            (
                pat(
                    DayPattern::DayOfWeek(vec![Item::Range(1, 7, 2)]),
                    MonthPattern::Wildcard,
                    YearPattern::Wildcard,
                ),
                "every 2nd day in Mon-Sun",
            ),
            (
                pat(
                    DayPattern::DayOfMonth(vec![Item::Range(1, 31, 2)]),
                    MonthPattern::Wildcard,
                    YearPattern::Wildcard,
                ),
                "every 2nd day in 1st-31st",
            ),
        ];

        for (input, expected) in cases {
            assert_eq!(strfrecur(&input), expected, "pattern: {:?}", input);
        }
    }

    #[test]
    fn format_day_fragment_lists() {
        let cases = [
            (
                pat(
                    DayPattern::DayOfWeek(vec![Item::Atom(1), Item::Atom(2)]),
                    MonthPattern::Wildcard,
                    YearPattern::Wildcard,
                ),
                "every Mon and Tue",
            ),
            (
                pat(
                    DayPattern::DayOfWeek(vec![Item::Atom(1), Item::Atom(3), Item::Atom(5)]),
                    MonthPattern::Wildcard,
                    YearPattern::Wildcard,
                ),
                "every Mon, Wed, and Fri",
            ),
            (
                pat(
                    DayPattern::DayOfMonth(vec![Item::Atom(1), Item::Atom(-1)]),
                    MonthPattern::Wildcard,
                    YearPattern::Wildcard,
                ),
                "every 1st and last",
            ),
        ];

        for (input, expected) in cases {
            assert_eq!(strfrecur(&input), expected, "pattern: {:?}", input);
        }
    }

    #[test]
    fn format_month_fragment_simplification() {
        let cases = [
            (
                pat(
                    DayPattern::Wildcard,
                    MonthPattern::List(vec![Item::Atom(1)]),
                    YearPattern::Wildcard,
                ),
                "every day in Jan",
            ),
            (
                pat(
                    DayPattern::Wildcard,
                    MonthPattern::List(vec![Item::Range(1, 6, 1)]),
                    YearPattern::Wildcard,
                ),
                "every day in Jan-Jun",
            ),
            (
                pat(
                    DayPattern::Wildcard,
                    MonthPattern::List(vec![Item::Range(1, 6, 2)]),
                    YearPattern::Wildcard,
                ),
                "every day in every 2nd month in Jan-Jun",
            ),
            (
                pat(
                    DayPattern::Wildcard,
                    MonthPattern::List(vec![Item::Range(1, 12, 2)]),
                    YearPattern::Wildcard,
                ),
                "every day in every 2nd month",
            ),
            (
                pat(
                    DayPattern::Wildcard,
                    MonthPattern::List(vec![Item::Atom(1), Item::Atom(7)]),
                    YearPattern::Wildcard,
                ),
                "every day in Jan and Jul",
            ),
        ];

        for (input, expected) in cases {
            assert_eq!(strfrecur(&input), expected, "pattern: {:?}", input);
        }
    }

    #[test]
    fn format_year_fragment_simplification() {
        let cases = [
            (
                pat(
                    DayPattern::Wildcard,
                    MonthPattern::Wildcard,
                    YearPattern::List(vec![Item::Atom(2026)]),
                ),
                "every day in 2026",
            ),
            (
                pat(
                    DayPattern::Wildcard,
                    MonthPattern::Wildcard,
                    YearPattern::List(vec![Item::Range(2026, 2030, 1)]),
                ),
                "every day in 2026-2030",
            ),
            (
                pat(
                    DayPattern::Wildcard,
                    MonthPattern::Wildcard,
                    YearPattern::List(vec![Item::Range(2026, 2030, 2)]),
                ),
                "every day in every 2nd year in 2026-2030",
            ),
            (
                pat(
                    DayPattern::Wildcard,
                    MonthPattern::Wildcard,
                    YearPattern::List(vec![Item::Range(1, 9999, 4)]),
                ),
                "every day in every 4th year",
            ),
            (
                pat(
                    DayPattern::Wildcard,
                    MonthPattern::List(vec![Item::Atom(1)]),
                    YearPattern::List(vec![Item::Atom(2026)]),
                ),
                "every day in Jan in 2026",
            ),
        ];

        for (input, expected) in cases {
            assert_eq!(strfrecur(&input), expected, "pattern: {:?}", input);
        }
    }

    #[test]
    fn format_normalizes_before_emitting() {
        let input = pat(
            DayPattern::DayOfWeek(vec![Item::Atom(5), Item::Atom(1), Item::Atom(5)]),
            MonthPattern::List(vec![Item::Range(1, 12, 1)]),
            YearPattern::Wildcard,
        );

        assert_eq!(strfrecur(&input), "every Mon and Fri");
    }
}
