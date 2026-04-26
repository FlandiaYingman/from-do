// TODO: inspect me

use winnow::{
    Parser, Result,
    ascii::{Caseless, digit1, space0, space1},
    combinator::{alt, opt, preceded, repeat},
    error::{ContextError, ParseError},
    token::take_while,
};

use super::pattern::{DayPattern, FieldUnit, Item, MonthPattern, Pattern, YearPattern};

pub fn strprecur<'a>(
    input: &'a str,
) -> std::result::Result<Pattern, ParseError<&'a str, ContextError>> {
    Pattern::parser.parse(input)
}

impl Pattern {
    fn parser<'s>(input: &mut &'s str) -> Result<Pattern> {
        (
            DayPattern::parser,
            opt(preceded(connector, MonthPattern::parser)),
            opt(preceded(connector, YearPattern::parser)),
        )
            .map(|(day, month, year)| {
                Pattern::new(
                    day,
                    month.unwrap_or(MonthPattern::Wildcard),
                    year.unwrap_or(YearPattern::Wildcard),
                )
                .normalized()
            })
            .parse_next(input)
    }
}

// Connector between fragments: required whitespace, optional "in".
fn connector<'s>(input: &mut &'s str) -> Result<()> {
    (space1, opt((Caseless("in"), space1)))
        .void()
        .parse_next(input)
}

// List separator. With a comma, surrounding whitespace and the literal "and"
// are all optional (the comma alone is enough to separate items). Without a
// comma, " and " (with surrounding whitespace) is required.
fn list_sep<'s>(input: &mut &'s str) -> Result<()> {
    alt((
        (',', space0, opt((Caseless("and"), space0))).void(),
        (space1, Caseless("and"), space1).void(),
    ))
    .parse_next(input)
}

impl DayPattern {
    fn parser<'s>(input: &mut &'s str) -> Result<DayPattern> {
        preceded(
            (Caseless("every"), space1),
            alt((
                Self::day_keyword_parser,
                Self::stepped_day_parser,
                Self::weekday_list_parser,
                Self::monthday_list_parser,
            )),
        )
        .parse_next(input)
    }

    // "day" -> Wildcard; "day in <weekday-range>" -> DayOfWeek(Range); "day in <md-range>" -> DayOfMonth(Range)
    fn day_keyword_parser<'s>(input: &mut &'s str) -> Result<DayPattern> {
        preceded(
            Caseless("day"),
            opt(preceded((space1, Caseless("in"), space1), day_range)),
        )
        .map(|range_opt| match range_opt {
            None => DayPattern::Wildcard,
            Some((FieldUnit::DayOfWeek, s, e)) => DayPattern::DayOfWeek(vec![Item::Range(s, e, 1)]),
            Some((_, s, e)) => DayPattern::DayOfMonth(vec![Item::Range(s, e, 1)]),
        })
        .parse_next(input)
    }

    // "<ord> day [in <range>]"
    fn stepped_day_parser<'s>(input: &mut &'s str) -> Result<DayPattern> {
        (
            positive_ordinal::<i16>,
            preceded(
                (space1, Caseless("day")),
                opt(preceded((space1, Caseless("in"), space1), day_range)),
            ),
        )
            .map(|(step, range_opt)| match range_opt {
                None => {
                    // Stepped wildcard with no bounds is ambiguous in canonical form,
                    // but English allows "every 2nd day" — assume MonthDay (1-31) since
                    // the formatter never emits this for day fragments without bounds.
                    DayPattern::DayOfMonth(vec![Item::Range(1, 31, step)])
                }
                Some((FieldUnit::DayOfWeek, s, e)) => {
                    DayPattern::DayOfWeek(vec![Item::Range(s, e, step)])
                }
                Some((_, s, e)) => DayPattern::DayOfMonth(vec![Item::Range(s, e, step)]),
            })
            .parse_next(input)
    }

    // "<weekday>" then optional list continuation of weekday items.
    fn weekday_list_parser<'s>(input: &mut &'s str) -> Result<DayPattern> {
        (
            weekday.map(Item::Atom),
            repeat(0.., preceded(list_sep, weekday_list_item)),
        )
            .map(|(first, rest): (Item, Vec<Item>)| {
                let mut items = vec![first];
                items.extend(rest);
                DayPattern::DayOfWeek(items)
            })
            .parse_next(input)
    }

    // "<monthday-atom>" then optional list continuation of monthday items.
    fn monthday_list_parser<'s>(input: &mut &'s str) -> Result<DayPattern> {
        (
            monthday_atom.map(Item::Atom),
            repeat(0.., preceded(list_sep, monthday_list_item)),
        )
            .map(|(first, rest): (Item, Vec<Item>)| {
                let mut items = vec![first];
                items.extend(rest);
                DayPattern::DayOfMonth(items)
            })
            .parse_next(input)
    }
}

fn weekday_list_item<'s>(input: &mut &'s str) -> Result<Item> {
    alt((
        // "day in <weekday-range>"
        preceded(
            (Caseless("day"), space1, Caseless("in"), space1),
            weekday_range,
        )
        .map(|(s, e)| Item::Range(s, e, 1)),
        // "<ord> day [in <weekday-range>]"
        (
            positive_ordinal::<i16>,
            preceded(
                (space1, Caseless("day")),
                opt(preceded((space1, Caseless("in"), space1), weekday_range)),
            ),
        )
            .map(|(step, range)| match range {
                Some((s, e)) => Item::Range(s, e, step),
                None => Item::Range(1, 7, step),
            }),
        weekday.map(Item::Atom),
    ))
    .parse_next(input)
}

fn monthday_list_item<'s>(input: &mut &'s str) -> Result<Item> {
    alt((
        preceded(
            (Caseless("day"), space1, Caseless("in"), space1),
            monthday_range,
        )
        .map(|(s, e)| Item::Range(s, e, 1)),
        (
            positive_ordinal::<i16>,
            preceded(
                (space1, Caseless("day")),
                opt(preceded((space1, Caseless("in"), space1), monthday_range)),
            ),
        )
            .map(|(step, range)| match range {
                Some((s, e)) => Item::Range(s, e, step),
                None => Item::Range(1, 31, step),
            }),
        monthday_atom.map(Item::Atom),
    ))
    .parse_next(input)
}

impl MonthPattern {
    fn parser<'s>(input: &mut &'s str) -> Result<MonthPattern> {
        alt((
            preceded((Caseless("every"), space1), Self::full_parser),
            month_range.map(|(s, e)| MonthPattern::List(vec![Item::Range(s, e, 1)])),
            month.map(|m| MonthPattern::List(vec![Item::Atom(m)])),
        ))
        .parse_next(input)
    }

    // Forms beginning with "every " already consumed.
    fn full_parser<'s>(input: &mut &'s str) -> Result<MonthPattern> {
        alt((
            // "month [in <range>]" + optional list tail
            (
                Caseless("month"),
                opt(preceded((space1, Caseless("in"), space1), month_range)),
            )
                .map(|(_, range)| match range {
                    None => MonthPattern::Wildcard,
                    Some((s, e)) => MonthPattern::List(vec![Item::Range(s, e, 1)]),
                })
                .and_then_month_list(month_list_item),
            // "<ord> month [in <range>]" + optional list tail
            (
                positive_ordinal::<i16>,
                preceded(
                    (space1, Caseless("month")),
                    opt(preceded((space1, Caseless("in"), space1), month_range)),
                ),
            )
                .map(|(step, range)| match range {
                    None => MonthPattern::List(vec![Item::Range(1, 12, step)]),
                    Some((s, e)) => MonthPattern::List(vec![Item::Range(s, e, step)]),
                })
                .and_then_month_list(month_list_item),
            // "<atom>" or "<range>" (with leading "every")
            month_range
                .map(|(s, e)| MonthPattern::List(vec![Item::Range(s, e, 1)]))
                .and_then_month_list(month_list_item),
            month
                .map(|m| MonthPattern::List(vec![Item::Atom(m)]))
                .and_then_month_list(month_list_item),
        ))
        .parse_next(input)
    }
}

impl YearPattern {
    fn parser<'s>(input: &mut &'s str) -> Result<YearPattern> {
        alt((
            preceded((Caseless("every"), space1), Self::full_parser),
            year_range.map(|(s, e)| YearPattern::List(vec![Item::Range(s, e, 1)])),
            year.map(|y| YearPattern::List(vec![Item::Atom(y)])),
        ))
        .parse_next(input)
    }

    fn full_parser<'s>(input: &mut &'s str) -> Result<YearPattern> {
        alt((
            (
                Caseless("year"),
                opt(preceded((space1, Caseless("in"), space1), year_range)),
            )
                .map(|(_, range)| match range {
                    None => YearPattern::Wildcard,
                    Some((s, e)) => YearPattern::List(vec![Item::Range(s, e, 1)]),
                })
                .and_then_year_list(year_list_item),
            (
                positive_ordinal::<i16>,
                preceded(
                    (space1, Caseless("year")),
                    opt(preceded((space1, Caseless("in"), space1), year_range)),
                ),
            )
                .map(|(step, range)| match range {
                    None => YearPattern::List(vec![Item::Range(1, 9999, step)]),
                    Some((s, e)) => YearPattern::List(vec![Item::Range(s, e, step)]),
                })
                .and_then_year_list(year_list_item),
            year_range
                .map(|(s, e)| YearPattern::List(vec![Item::Range(s, e, 1)]))
                .and_then_year_list(year_list_item),
            year.map(|y| YearPattern::List(vec![Item::Atom(y)]))
                .and_then_year_list(year_list_item),
        ))
        .parse_next(input)
    }
}

// --- list-tail extension traits -------------------------------------------
// Helper to chain a leading-fragment parser with a `,` / `and` separated list tail.

trait ParserExtMonth<'s>: Parser<&'s str, MonthPattern, ContextError> + Sized {
    fn and_then_month_list<F>(self, item: F) -> impl Parser<&'s str, MonthPattern, ContextError>
    where
        F: for<'b> FnMut(&mut &'b str) -> Result<Item> + Copy + 's,
    {
        (self, repeat(0.., preceded(list_sep, item))).map(
            |(first, rest): (MonthPattern, Vec<Item>)| {
                let mut items = match first {
                    MonthPattern::Wildcard => return MonthPattern::Wildcard,
                    MonthPattern::List(v) => v,
                };
                items.extend(rest);
                MonthPattern::List(items)
            },
        )
    }
}
impl<'s, T: Parser<&'s str, MonthPattern, ContextError>> ParserExtMonth<'s> for T {}

trait ParserExtYear<'s>: Parser<&'s str, YearPattern, ContextError> + Sized {
    fn and_then_year_list<F>(self, item: F) -> impl Parser<&'s str, YearPattern, ContextError>
    where
        F: for<'b> FnMut(&mut &'b str) -> Result<Item> + Copy + 's,
    {
        (self, repeat(0.., preceded(list_sep, item))).map(
            |(first, rest): (YearPattern, Vec<Item>)| {
                let mut items = match first {
                    YearPattern::Wildcard => return YearPattern::Wildcard,
                    YearPattern::List(v) => v,
                };
                items.extend(rest);
                YearPattern::List(items)
            },
        )
    }
}
impl<'s, T: Parser<&'s str, YearPattern, ContextError>> ParserExtYear<'s> for T {}

fn month_list_item<'s>(input: &mut &'s str) -> Result<Item> {
    alt((
        preceded(
            (Caseless("month"), space1, Caseless("in"), space1),
            month_range,
        )
        .map(|(s, e)| Item::Range(s, e, 1)),
        (
            positive_ordinal::<i16>,
            preceded(
                (space1, Caseless("month")),
                opt(preceded((space1, Caseless("in"), space1), month_range)),
            ),
        )
            .map(|(step, range)| match range {
                Some((s, e)) => Item::Range(s, e, step),
                None => Item::Range(1, 12, step),
            }),
        month_range.map(|(s, e)| Item::Range(s, e, 1)),
        month.map(Item::Atom),
    ))
    .parse_next(input)
}

fn year_list_item<'s>(input: &mut &'s str) -> Result<Item> {
    alt((
        preceded(
            (Caseless("year"), space1, Caseless("in"), space1),
            year_range,
        )
        .map(|(s, e)| Item::Range(s, e, 1)),
        (
            positive_ordinal::<i16>,
            preceded(
                (space1, Caseless("year")),
                opt(preceded((space1, Caseless("in"), space1), year_range)),
            ),
        )
            .map(|(step, range)| match range {
                Some((s, e)) => Item::Range(s, e, step),
                None => Item::Range(1, 9999, step),
            }),
        year_range.map(|(s, e)| Item::Range(s, e, 1)),
        year.map(Item::Atom),
    ))
    .parse_next(input)
}

// --- atomic parsers --------------------------------------------------------

fn day_range<'s>(input: &mut &'s str) -> Result<(FieldUnit, i16, i16)> {
    alt((
        weekday_range.map(|(s, e)| (FieldUnit::DayOfWeek, s, e)),
        monthday_range.map(|(s, e)| (FieldUnit::DayOfMonth, s, e)),
    ))
    .parse_next(input)
}

fn weekday_range<'s>(input: &mut &'s str) -> Result<(i16, i16)> {
    (weekday, '-', weekday)
        .map(|(s, _, e)| (s, e))
        .parse_next(input)
}

fn monthday_range<'s>(input: &mut &'s str) -> Result<(i16, i16)> {
    (monthday_atom, '-', monthday_atom)
        .map(|(s, _, e)| (s, e))
        .parse_next(input)
}

fn month_range<'s>(input: &mut &'s str) -> Result<(i16, i16)> {
    (month, '-', month)
        .map(|(s, _, e)| (s, e))
        .parse_next(input)
}

fn year_range<'s>(input: &mut &'s str) -> Result<(i16, i16)> {
    (year, '-', year).map(|(s, _, e)| (s, e)).parse_next(input)
}

fn weekday<'s>(input: &mut &'s str) -> Result<i16> {
    take_while(1.., |c: char| c.is_ascii_alphabetic())
        .verify_map(parse_weekday_prefix)
        .parse_next(input)
}

fn month<'s>(input: &mut &'s str) -> Result<i16> {
    take_while(1.., |c: char| c.is_ascii_alphabetic())
        .verify_map(parse_month_prefix)
        .parse_next(input)
}

fn year<'s>(input: &mut &'s str) -> Result<i16> {
    digit1
        .try_map(|s: &str| s.parse::<i16>())
        .verify(|n| (1..=9999).contains(n))
        .parse_next(input)
}

fn monthday_atom<'s>(input: &mut &'s str) -> Result<i16> {
    alt((
        Caseless("last").value(-1_i16),
        (signed_ordinal, opt(preceded(space1, Caseless("last"))))
            .map(|(n, last)| if last.is_some() { -n.abs() } else { n }),
    ))
    .parse_next(input)
}

fn signed_ordinal<'s>(input: &mut &'s str) -> Result<i16> {
    (opt('-'), digit1, ordinal_suffix)
        .verify_map(|(sign, digits, _): (_, &str, _)| {
            let n = digits.parse::<i16>().ok()?;
            Some(if sign.is_some() { -n } else { n })
        })
        .parse_next(input)
}

fn positive_ordinal<'s, N: TryFrom<u32>>(input: &mut &'s str) -> Result<N> {
    (digit1, ordinal_suffix)
        .verify_map(|(digits, _): (&str, _)| {
            let n = digits.parse::<u32>().ok()?;
            if n == 0 {
                return None;
            }
            N::try_from(n).ok()
        })
        .parse_next(input)
}

fn ordinal_suffix<'s>(input: &mut &'s str) -> Result<()> {
    alt((
        Caseless("st"),
        Caseless("nd"),
        Caseless("rd"),
        Caseless("th"),
    ))
    .void()
    .parse_next(input)
}

const WEEKDAYS: [(&str, i16); 7] = [
    ("monday", 1),
    ("tuesday", 2),
    ("wednesday", 3),
    ("thursday", 4),
    ("friday", 5),
    ("saturday", 6),
    ("sunday", 7),
];

const MONTHS: [(&str, i16); 12] = [
    ("january", 1),
    ("february", 2),
    ("march", 3),
    ("april", 4),
    ("may", 5),
    ("june", 6),
    ("july", 7),
    ("august", 8),
    ("september", 9),
    ("october", 10),
    ("november", 11),
    ("december", 12),
];

fn parse_weekday_prefix(token: &str) -> Option<i16> {
    parse_unique_prefix(token, &WEEKDAYS)
}

fn parse_month_prefix(token: &str) -> Option<i16> {
    parse_unique_prefix(token, &MONTHS)
}

fn parse_unique_prefix(token: &str, table: &[(&str, i16)]) -> Option<i16> {
    let token = token.to_ascii_lowercase();
    let mut iter = table
        .iter()
        .filter(|(full, _)| full.starts_with(&token))
        .map(|(_, value)| *value);
    let first = iter.next()?;
    iter.next().is_none().then_some(first)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pat(d: DayPattern, m: MonthPattern, y: YearPattern) -> Pattern {
        Pattern::new(d, m, y)
    }

    #[test]
    fn parse_day_fragment_canonical() {
        let cases = [
            (
                "every day",
                pat(
                    DayPattern::Wildcard,
                    MonthPattern::Wildcard,
                    YearPattern::Wildcard,
                ),
            ),
            (
                "every Mon",
                pat(
                    DayPattern::DayOfWeek(vec![Item::Atom(1)]),
                    MonthPattern::Wildcard,
                    YearPattern::Wildcard,
                ),
            ),
            (
                "every 1st",
                pat(
                    DayPattern::DayOfMonth(vec![Item::Atom(1)]),
                    MonthPattern::Wildcard,
                    YearPattern::Wildcard,
                ),
            ),
            (
                "every last",
                pat(
                    DayPattern::DayOfMonth(vec![Item::Atom(-1)]),
                    MonthPattern::Wildcard,
                    YearPattern::Wildcard,
                ),
            ),
            (
                "every 2nd last",
                pat(
                    DayPattern::DayOfMonth(vec![Item::Atom(-2)]),
                    MonthPattern::Wildcard,
                    YearPattern::Wildcard,
                ),
            ),
            (
                "every day in Mon-Fri",
                pat(
                    DayPattern::DayOfWeek(vec![Item::Range(1, 5, 1)]),
                    MonthPattern::Wildcard,
                    YearPattern::Wildcard,
                ),
            ),
            (
                "every 2nd day",
                pat(
                    DayPattern::DayOfMonth(vec![Item::Range(1, 31, 2)]),
                    MonthPattern::Wildcard,
                    YearPattern::Wildcard,
                ),
            ),
        ];

        for (input, expected) in cases {
            assert_eq!(strprecur(input).unwrap(), expected, "input: {input}");
        }
    }

    #[test]
    fn parse_day_fragment_lists() {
        let cases = [
            (
                "every Mon and Tue",
                pat(
                    DayPattern::DayOfWeek(vec![Item::Atom(1), Item::Atom(2)]),
                    MonthPattern::Wildcard,
                    YearPattern::Wildcard,
                ),
            ),
            (
                "every Mon, Wed, and Fri",
                pat(
                    DayPattern::DayOfWeek(vec![Item::Atom(1), Item::Atom(3), Item::Atom(5)]),
                    MonthPattern::Wildcard,
                    YearPattern::Wildcard,
                ),
            ),
            (
                "every 1st and last",
                pat(
                    DayPattern::DayOfMonth(vec![Item::Atom(1), Item::Atom(-1)]),
                    MonthPattern::Wildcard,
                    YearPattern::Wildcard,
                ),
            ),
        ];

        for (input, expected) in cases {
            assert_eq!(strprecur(input).unwrap(), expected, "input: {input}");
        }
    }

    #[test]
    fn parse_month_fragment_simplifications() {
        let cases = [
            (
                "every day in Jan",
                pat(
                    DayPattern::Wildcard,
                    MonthPattern::List(vec![Item::Atom(1)]),
                    YearPattern::Wildcard,
                ),
            ),
            (
                "every day in Jan-Jun",
                pat(
                    DayPattern::Wildcard,
                    MonthPattern::List(vec![Item::Range(1, 6, 1)]),
                    YearPattern::Wildcard,
                ),
            ),
            (
                "every day in every 2nd month in Jan-Jun",
                pat(
                    DayPattern::Wildcard,
                    MonthPattern::List(vec![Item::Range(1, 6, 2)]),
                    YearPattern::Wildcard,
                ),
            ),
            (
                "every day in every 2nd month",
                pat(
                    DayPattern::Wildcard,
                    MonthPattern::List(vec![Item::Range(1, 12, 2)]),
                    YearPattern::Wildcard,
                ),
            ),
        ];

        for (input, expected) in cases {
            assert_eq!(strprecur(input).unwrap(), expected, "input: {input}");
        }
    }

    #[test]
    fn parse_year_fragment_canonical() {
        let cases = [
            (
                "every day in 2026",
                pat(
                    DayPattern::Wildcard,
                    MonthPattern::Wildcard,
                    YearPattern::List(vec![Item::Atom(2026)]),
                ),
            ),
            (
                "every day in 2026-2030",
                pat(
                    DayPattern::Wildcard,
                    MonthPattern::Wildcard,
                    YearPattern::List(vec![Item::Range(2026, 2030, 1)]),
                ),
            ),
            (
                "every day in every 2nd year in 2026-2030",
                pat(
                    DayPattern::Wildcard,
                    MonthPattern::Wildcard,
                    YearPattern::List(vec![Item::Range(2026, 2030, 2)]),
                ),
            ),
            (
                "every day in every 4th year",
                pat(
                    DayPattern::Wildcard,
                    MonthPattern::Wildcard,
                    YearPattern::List(vec![Item::Range(1, 9999, 4)]),
                ),
            ),
            (
                "every day in Jan in 2026",
                pat(
                    DayPattern::Wildcard,
                    MonthPattern::List(vec![Item::Atom(1)]),
                    YearPattern::List(vec![Item::Atom(2026)]),
                ),
            ),
        ];

        for (input, expected) in cases {
            assert_eq!(strprecur(input).unwrap(), expected, "input: {input}");
        }
    }

    #[test]
    fn parse_is_case_insensitive() {
        assert_eq!(
            strprecur("EVERY MONDAY IN JAN IN 2026").unwrap(),
            pat(
                DayPattern::DayOfWeek(vec![Item::Atom(1)]),
                MonthPattern::List(vec![Item::Atom(1)]),
                YearPattern::List(vec![Item::Atom(2026)]),
            )
        );
    }

    #[test]
    fn parse_accepts_weekday_and_month_prefixes() {
        let cases = [
            (
                "every Mond",
                pat(
                    DayPattern::DayOfWeek(vec![Item::Atom(1)]),
                    MonthPattern::Wildcard,
                    YearPattern::Wildcard,
                ),
            ),
            (
                "every day in Mar",
                pat(
                    DayPattern::Wildcard,
                    MonthPattern::List(vec![Item::Atom(3)]),
                    YearPattern::Wildcard,
                ),
            ),
            (
                "every day in Marc",
                pat(
                    DayPattern::Wildcard,
                    MonthPattern::List(vec![Item::Atom(3)]),
                    YearPattern::Wildcard,
                ),
            ),
        ];

        for (input, expected) in cases {
            assert_eq!(strprecur(input).unwrap(), expected, "input: {input}");
        }
    }

    #[test]
    fn parse_rejects_ambiguous_prefixes() {
        // "M" matches both March and May.
        assert!(strprecur("every day in M").is_err());
        // "T" matches both Tuesday and Thursday.
        assert!(strprecur("every T").is_err());
    }

    #[test]
    fn parse_accepts_relaxed_list_separators() {
        let expected = pat(
            DayPattern::DayOfWeek(vec![Item::Atom(1), Item::Atom(3), Item::Atom(5)]),
            MonthPattern::Wildcard,
            YearPattern::Wildcard,
        );
        let inputs = [
            "every Mon, Wed, and Fri",
            "every Mon, Wed, Fri",
            "every Mon,Wed,Fri",
            "every Mon,Wed,and Fri",
            "every Mon, Wed and Fri",
            "every Mon and Wed and Fri",
            "every Mon,Wed,andFri",
        ];
        for input in inputs {
            assert_eq!(strprecur(input).unwrap(), expected, "input: {input}");
        }
    }
}
