use jiff::civil::{Date, Weekday};
use winnow::{
    Parser, Result,
    ascii::{Caseless, digit1, space1},
    combinator::{alt, opt, preceded, seq},
    error::{ContextError, ParseError},
    token::take_while,
};

use super::phrase::{
    CalendarOffset, CalendarUnit, DateSpec, Direction, Phrase, Relation, TimeSpec,
};

pub fn strpcur<'a>(input: &'a str) -> Result<Phrase, ParseError<&'a str, ContextError>> {
    Phrase::parser.parse(input)
}

impl Phrase {
    fn parser<'s>(input: &mut &'s str) -> Result<Phrase> {
        alt((Self::calendar_offset_parser, Self::datetime_parser)).parse_next(input)
    }

    fn datetime_parser<'s>(input: &mut &'s str) -> Result<Phrase> {
        (opt(DateSpec::parser), opt(space1), opt(TimeSpec::parser))
            .verify(|(date_spec, _, time_spec)| date_spec.is_some() || time_spec.is_some())
            .map(|(date_spec, _, time_spec)| Phrase::DateTime {
                date: date_spec.unwrap_or(DateSpec::Today),
                time: time_spec.unwrap_or(TimeSpec::Unspecified),
            })
            .parse_next(input)
    }

    fn calendar_offset_parser<'s>(input: &mut &'s str) -> Result<Phrase> {
        CalendarOffset::parser
            .map(Phrase::CalendarOffset)
            .parse_next(input)
    }
}

impl DateSpec {
    fn parser<'s>(input: &mut &'s str) -> Result<DateSpec> {
        alt((
            Self::day_parser,
            Self::weekday_parser,
            Self::absolute_parser,
        ))
        .parse_next(input)
    }

    fn day_parser<'s>(input: &mut &'s str) -> Result<DateSpec> {
        alt((
            Caseless("yesterday").value(DateSpec::Yesterday),
            Caseless("today").value(DateSpec::Today),
            Caseless("tomorrow").value(DateSpec::Tomorrow),
        ))
        .parse_next(input)
    }

    fn weekday_parser<'s>(input: &mut &'s str) -> Result<DateSpec> {
        let relation = alt((
            Caseless("last").value(Relation::Last),
            Caseless("this").value(Relation::This),
            Caseless("next").value(Relation::Next),
        ));
        let weekday = alt((
            Caseless("Monday").value(Weekday::Monday),
            Caseless("Tuesday").value(Weekday::Tuesday),
            Caseless("Wednesday").value(Weekday::Wednesday),
            Caseless("Thursday").value(Weekday::Thursday),
            Caseless("Friday").value(Weekday::Friday),
            Caseless("Saturday").value(Weekday::Saturday),
            Caseless("Sunday").value(Weekday::Sunday),
        ));
        (relation, space1, weekday)
            .map(|(relation, _, weekday)| DateSpec::Weekday(relation, weekday))
            .parse_next(input)
    }

    fn absolute_parser<'s>(input: &mut &'s str) -> Result<DateSpec> {
        (Caseless("on"), space1, d, space1, m, space1, y)
            .map(|(_, _, d, _, m, _, y)| DateSpec::Absolute(Date::new(y, m, d).unwrap()))
            .parse_next(input)
    }
}

impl TimeSpec {
    fn parser<'s>(input: &mut &'s str) -> Result<TimeSpec> {
        (
            Caseless("at"),
            space1,
            hr,
            opt(preceded(':', (min, opt(preceded(':', sec))))),
            opt(preceded(space1, meridiem)),
        )
            .map(|(_, _, hr, rest, meridiem)| {
                let mut hr = hr;
                let mut min = 0;
                let mut sec = 0;
                if let Some((min_v, _)) = rest {
                    min = min_v;
                }
                if let Some((_, Some(sec_v))) = rest {
                    sec = sec_v;
                }

                match meridiem {
                    Some(meridiem) => {
                        hr = match (hr, meridiem) {
                            (12, "AM") => 0,
                            (12, "PM") => 12,
                            (h, "AM") => h,
                            (h, "PM") => h + 12,
                            _ => unreachable!(),
                        };
                        TimeSpec::Absolute(jiff::civil::Time::new(hr, min, sec, 0).unwrap())
                    }
                    None => TimeSpec::Absolute(jiff::civil::Time::new(hr, min, sec, 0).unwrap()),
                }
            })
            .parse_next(input)
    }
}

impl CalendarOffset {
    fn parser<'s>(input: &mut &'s str) -> Result<CalendarOffset> {
        alt((Self::past_parser, Self::future_parser)).parse_next(input)
    }

    fn past_parser<'s>(input: &mut &'s str) -> Result<CalendarOffset> {
        seq! {CalendarOffset{
            amount: int.verify(|value| *value > 0),
            _: space1,
            unit: CalendarUnit::parser,
            _: space1,
            direction: Caseless("ago").value(Direction::Past),
        }}
        .parse_next(input)
    }

    fn future_parser<'s>(input: &mut &'s str) -> Result<CalendarOffset> {
        seq! {CalendarOffset{
            direction: Caseless("in").value(Direction::Future),
            _: space1,
            amount: int.verify(|value| *value > 0),
            _: space1,
            unit: CalendarUnit::parser,
        }}
        .parse_next(input)
    }
}

impl CalendarUnit {
    fn parser<'s>(input: &mut &'s str) -> Result<CalendarUnit> {
        alt((
            alt((Caseless("days"), Caseless("day"), Caseless("d"))).value(CalendarUnit::Day),
            alt((Caseless("weeks"), Caseless("week"), Caseless("w"))).value(CalendarUnit::Week),
            alt((Caseless("months"), Caseless("month"))).value(CalendarUnit::Month),
            alt((Caseless("years"), Caseless("year"))).value(CalendarUnit::Year),
        ))
        .parse_next(input)
    }
}

fn int<'s>(input: &mut &'s str) -> Result<i64> {
    digit1
        .parse_to()
        .verify(|value: &i64| *value >= 0)
        .parse_next(input)
}

fn d<'s>(input: &mut &'s str) -> Result<i8> {
    int.verify(|value| (1..=31).contains(value))
        .map(|v| v as i8)
        .parse_next(input)
}

fn m<'s>(input: &mut &'s str) -> Result<i8> {
    alt((
        alt((
            Caseless("Jan").value(1),
            Caseless("Feb").value(2),
            Caseless("Mar").value(3),
            Caseless("Apr").value(4),
            Caseless("May").value(5),
            Caseless("Jun").value(6),
        )),
        alt((
            Caseless("Jul").value(7),
            Caseless("Aug").value(8),
            Caseless("Sep").value(9),
            Caseless("Oct").value(10),
            Caseless("Nov").value(11),
            Caseless("Dec").value(12),
        )),
    ))
    .parse_next(input)
}

fn y<'s>(input: &mut &'s str) -> Result<i16> {
    take_while(4usize, |c: char| c.is_ascii_digit())
        .parse_to()
        .parse_next(input)
}

fn hr<'s>(input: &mut &'s str) -> Result<i8> {
    int.verify(|v| (0..=23).contains(v))
        .map(|v| v as i8)
        .parse_next(input)
}

fn min<'s>(input: &mut &'s str) -> Result<i8> {
    int.verify(|v| (0..=59).contains(v))
        .map(|v| v as i8)
        .parse_next(input)
}

fn sec<'s>(input: &mut &'s str) -> Result<i8> {
    int.verify(|v| (0..=59).contains(v))
        .map(|v| v as i8)
        .parse_next(input)
}

fn meridiem<'s>(input: &mut &'s str) -> Result<&'static str> {
    alt((Caseless("AM").value("AM"), Caseless("PM").value("PM"))).parse_next(input)
}

#[cfg(test)]
mod tests {
    use super::*;
    use jiff::Zoned;

    #[test]
    fn parse_date_spec_relative_days() {
        let cases = [
            ("yesterday", "2026-04-06T16:42:00+00:00[UTC]"),
            ("today", "2026-04-07T16:42:00+00:00[UTC]"),
            ("tomorrow", "2026-04-08T16:42:00+00:00[UTC]"),
        ];

        let reference = "2026-04-07T16:42:00+00:00[UTC]".parse::<Zoned>().unwrap();
        for (input, expected) in cases {
            assert_eq!(
                strpcur(input).unwrap().resolve(&reference),
                expected.parse::<Zoned>().unwrap(),
                "input: {input}",
            );
        }
    }

    #[test]
    fn parse_date_spec_relative_weekdays() {
        let cases = [
            ("last Monday", "2026-03-30T16:42:00+00:00[UTC]"),
            ("last Tuesday", "2026-03-31T16:42:00+00:00[UTC]"),
            ("last Wednesday", "2026-04-01T16:42:00+00:00[UTC]"),
            ("last Thursday", "2026-04-02T16:42:00+00:00[UTC]"),
            ("last Friday", "2026-04-03T16:42:00+00:00[UTC]"),
            ("last Saturday", "2026-04-04T16:42:00+00:00[UTC]"),
            ("last Sunday", "2026-04-05T16:42:00+00:00[UTC]"),
            ("this Monday", "2026-04-06T16:42:00+00:00[UTC]"),
            ("this Tuesday", "2026-04-07T16:42:00+00:00[UTC]"),
            ("this Wednesday", "2026-04-08T16:42:00+00:00[UTC]"),
            ("this Thursday", "2026-04-09T16:42:00+00:00[UTC]"),
            ("this Friday", "2026-04-10T16:42:00+00:00[UTC]"),
            ("this Saturday", "2026-04-11T16:42:00+00:00[UTC]"),
            ("this Sunday", "2026-04-12T16:42:00+00:00[UTC]"),
            ("next Monday", "2026-04-13T16:42:00+00:00[UTC]"),
            ("next Tuesday", "2026-04-14T16:42:00+00:00[UTC]"),
            ("next Wednesday", "2026-04-15T16:42:00+00:00[UTC]"),
            ("next Thursday", "2026-04-16T16:42:00+00:00[UTC]"),
            ("next Friday", "2026-04-17T16:42:00+00:00[UTC]"),
            ("next Saturday", "2026-04-18T16:42:00+00:00[UTC]"),
            ("next Sunday", "2026-04-19T16:42:00+00:00[UTC]"),
        ];

        let reference = "2026-04-07T16:42:00+00:00[UTC]".parse::<Zoned>().unwrap();
        for (input, expected) in cases {
            assert_eq!(
                strpcur(input).unwrap().resolve(&reference),
                expected.parse::<Zoned>().unwrap(),
                "input: {input}",
            );
        }
    }

    #[test]
    fn parse_date_spec_absolute() {
        let cases = [
            ("on 29 Feb 2024", "2024-02-29T16:42:00+00:00[UTC]"),
            ("on 20 Apr 2026", "2026-04-20T16:42:00+00:00[UTC]"),
            ("on 31 Dec 2030", "2030-12-31T16:42:00+00:00[UTC]"),
        ];

        let reference = "2026-04-07T16:42:00+00:00[UTC]".parse::<Zoned>().unwrap();
        for (input, expected) in cases {
            assert_eq!(
                strpcur(input).unwrap().resolve(&reference),
                expected.parse::<Zoned>().unwrap(),
                "input: {input}",
            );
        }
    }

    #[test]
    fn parse_time_spec_24_hour() {
        let cases = [
            ("at 00", "2026-04-07T00:00:00+00:00[UTC]"),
            ("at 09", "2026-04-07T09:00:00+00:00[UTC]"),
            ("at 09:30", "2026-04-07T09:30:00+00:00[UTC]"),
            ("at 09:30:45", "2026-04-07T09:30:45+00:00[UTC]"),
            ("at 21:30:45", "2026-04-07T21:30:45+00:00[UTC]"),
        ];

        let reference = "2026-04-07T16:42:00+00:00[UTC]".parse::<Zoned>().unwrap();
        for (input, expected) in cases {
            assert_eq!(
                strpcur(input).unwrap().resolve(&reference),
                expected.parse::<Zoned>().unwrap(),
                "input: {input}",
            );
        }
    }

    #[test]
    fn parse_time_spec_12_hour() {
        let cases = [
            ("at 12 AM", "2026-04-07T00:00:00+00:00[UTC]"),
            ("at 12 PM", "2026-04-07T12:00:00+00:00[UTC]"),
            ("at 9 AM", "2026-04-07T09:00:00+00:00[UTC]"),
            ("at 9:30 PM", "2026-04-07T21:30:00+00:00[UTC]"),
            ("at 9:30:45 PM", "2026-04-07T21:30:45+00:00[UTC]"),
        ];

        let reference = "2026-04-07T16:42:00+00:00[UTC]".parse::<Zoned>().unwrap();
        for (input, expected) in cases {
            assert_eq!(
                strpcur(input).unwrap().resolve(&reference),
                expected.parse::<Zoned>().unwrap(),
                "input: {input}",
            );
        }
    }

    #[test]
    fn parse_datetime_spec() {
        let cases = [
            ("tomorrow at 09", "2026-04-08T09:00:00+00:00[UTC]"),
            ("this Friday at 09:30", "2026-04-10T09:30:00+00:00[UTC]"),
            (
                "next Monday at 9:30:45 PM",
                "2026-04-13T21:30:45+00:00[UTC]",
            ),
            ("on 21 May 2026", "2026-05-21T16:42:00+00:00[UTC]"),
            (
                "on 21 May 2026 at 09:30:45",
                "2026-05-21T09:30:45+00:00[UTC]",
            ),
            (
                "on 21 May 2026 at 9:30 PM",
                "2026-05-21T21:30:00+00:00[UTC]",
            ),
        ];

        let reference = "2026-04-07T16:42:00+00:00[UTC]".parse::<Zoned>().unwrap();
        for (input, expected) in cases {
            assert_eq!(
                strpcur(input).unwrap().resolve(&reference),
                expected.parse::<Zoned>().unwrap(),
                "input: {input}",
            );
        }
    }

    #[test]
    fn parse_calendar_offset() {
        let cases = [
            (
                "in 13 days",
                "2026-04-07T16:42:00+00:00[UTC]",
                "2026-04-20T16:42:00+00:00[UTC]",
            ),
            (
                "13 days ago",
                "2026-04-07T16:42:00+00:00[UTC]",
                "2026-03-25T16:42:00+00:00[UTC]",
            ),
            (
                "in 2 weeks",
                "2026-04-07T16:42:00+00:00[UTC]",
                "2026-04-21T16:42:00+00:00[UTC]",
            ),
            (
                "2 weeks ago",
                "2026-04-07T16:42:00+00:00[UTC]",
                "2026-03-24T16:42:00+00:00[UTC]",
            ),
            (
                "in 1 month",
                "2026-04-07T16:42:00+00:00[UTC]",
                "2026-05-07T16:42:00+00:00[UTC]",
            ),
            (
                "1 month ago",
                "2026-04-07T16:42:00+00:00[UTC]",
                "2026-03-07T16:42:00+00:00[UTC]",
            ),
            (
                "in 2 months",
                "2026-04-07T16:42:00+00:00[UTC]",
                "2026-06-07T16:42:00+00:00[UTC]",
            ),
            (
                "2 months ago",
                "2026-04-07T16:42:00+00:00[UTC]",
                "2026-02-07T16:42:00+00:00[UTC]",
            ),
            (
                "in 1 month",
                "2026-01-31T16:42:00+00:00[UTC]",
                "2026-02-28T16:42:00+00:00[UTC]",
            ),
            (
                "1 month ago",
                "2026-03-31T16:42:00+00:00[UTC]",
                "2026-02-28T16:42:00+00:00[UTC]",
            ),
            (
                "in 1 year",
                "2026-04-07T16:42:00+00:00[UTC]",
                "2027-04-07T16:42:00+00:00[UTC]",
            ),
            (
                "1 year ago",
                "2026-04-07T16:42:00+00:00[UTC]",
                "2025-04-07T16:42:00+00:00[UTC]",
            ),
            (
                "in 2 years",
                "2026-04-07T16:42:00+00:00[UTC]",
                "2028-04-07T16:42:00+00:00[UTC]",
            ),
            (
                "2 years ago",
                "2026-04-07T16:42:00+00:00[UTC]",
                "2024-04-07T16:42:00+00:00[UTC]",
            ),
            (
                "in 1 year",
                "2024-02-29T16:42:00+00:00[UTC]",
                "2025-02-28T16:42:00+00:00[UTC]",
            ),
            (
                "1 year ago",
                "2024-02-29T16:42:00+00:00[UTC]",
                "2023-02-28T16:42:00+00:00[UTC]",
            ),
        ];

        for (input, reference, expected) in cases {
            assert_eq!(
                strpcur(input)
                    .unwrap()
                    .resolve(&reference.parse::<Zoned>().unwrap()),
                expected.parse::<Zoned>().unwrap(),
                "input: {input}; reference: {reference}",
            );
        }
    }

    #[test]
    fn parse_calendar_offset_short_units() {
        let cases = [
            ("in 13 d", "2026-04-20T16:42:00+00:00[UTC]"),
            ("13 day ago", "2026-03-25T16:42:00+00:00[UTC]"),
            ("in 2 w", "2026-04-21T16:42:00+00:00[UTC]"),
            ("2 week ago", "2026-03-24T16:42:00+00:00[UTC]"),
        ];

        let reference = "2026-04-07T16:42:00+00:00[UTC]".parse::<Zoned>().unwrap();
        for (input, expected) in cases {
            assert_eq!(
                strpcur(input).unwrap().resolve(&reference),
                expected.parse::<Zoned>().unwrap(),
                "input: {input}",
            );
        }
    }

    #[test]
    fn parse_case_insensitive() {
        let cases = [
            ("TODAY", "2026-04-07T16:42:00+00:00[UTC]"),
            ("ToMoRrOw At 09", "2026-04-08T09:00:00+00:00[UTC]"),
            ("ThIs FrIdAy At 9:30 Pm", "2026-04-10T21:30:00+00:00[UTC]"),
            (
                "ON 21 MAY 2026 AT 09:30:45",
                "2026-05-21T09:30:45+00:00[UTC]",
            ),
            ("In 2 WeEkS", "2026-04-21T16:42:00+00:00[UTC]"),
            ("13 D aGo", "2026-03-25T16:42:00+00:00[UTC]"),
        ];

        let reference = "2026-04-07T16:42:00+00:00[UTC]".parse::<Zoned>().unwrap();
        for (input, expected) in cases {
            assert_eq!(
                strpcur(input).unwrap().resolve(&reference),
                expected.parse::<Zoned>().unwrap(),
                "input: {input}",
            );
        }
    }
}
