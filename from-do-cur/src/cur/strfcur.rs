use core::panic;

use jiff::civil::Weekday;

use super::phrase::*;

pub fn strfcur(target: &Phrase) -> String {
    match target {
        Phrase::DateTime { date, time } => {
            let date_str = date.format();
            let time_str = time.format();
            format!("{} {}", date_str, time_str).trim().to_string()
        }
        Phrase::CalendarOffset(offset) => offset.format(),
    }
}

impl CalendarOffset {
    fn format(&self) -> String {
        let unit = match self.unit {
            CalendarUnit::Day => pluralize(self.amount, "day", "days"),
            CalendarUnit::Week => pluralize(self.amount, "week", "weeks"),
            CalendarUnit::Month => pluralize(self.amount, "month", "months"),
            CalendarUnit::Year => pluralize(self.amount, "year", "years"),
        };

        match self.direction {
            Direction::Future => format!("in {} {}", self.amount, unit),
            Direction::Past => format!("{} {} ago", self.amount, unit),
        }
    }
}

impl DateSpec {
    fn format_relation(relation: &Relation) -> &'static str {
        match relation {
            Relation::Last => "last",
            Relation::This => "this",
            Relation::Next => "next",
        }
    }

    fn format_weekday(weekday: &Weekday) -> &'static str {
        match weekday {
            Weekday::Monday => "Monday",
            Weekday::Tuesday => "Tuesday",
            Weekday::Wednesday => "Wednesday",
            Weekday::Thursday => "Thursday",
            Weekday::Friday => "Friday",
            Weekday::Saturday => "Saturday",
            Weekday::Sunday => "Sunday",
        }
    }

    fn format_month(month: i8) -> &'static str {
        match month {
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
            _ => panic!("invalid month: {}", month),
        }
    }

    fn format(&self) -> String {
        match self {
            DateSpec::Today => "today".to_string(),
            DateSpec::Tomorrow => "tomorrow".to_string(),
            DateSpec::Yesterday => "yesterday".to_string(),
            DateSpec::Weekday(relation, weekday) => {
                format!(
                    "{} {}",
                    Self::format_relation(relation),
                    Self::format_weekday(weekday)
                )
            }
            DateSpec::Absolute(date) => {
                format!(
                    "on {} {} {}",
                    date.day(),
                    Self::format_month(date.month()),
                    date.year()
                )
            }
        }
    }
}

impl TimeSpec {
    fn format(&self) -> String {
        match self {
            TimeSpec::Absolute(time) if time.minute() == 0 && time.second() == 0 => {
                format!("at {:02}", time.hour())
            }
            TimeSpec::Absolute(time) if time.second() == 0 => {
                format!("at {:02}:{:02}", time.hour(), time.minute())
            }
            TimeSpec::Absolute(time) => {
                format!(
                    "at {:02}:{:02}:{:02}",
                    time.hour(),
                    time.minute(),
                    time.second()
                )
            }
            TimeSpec::Unspecified => "".to_string(),
        }
    }
}

fn pluralize<'a>(amount: i64, singular: &'a str, plural: &'a str) -> &'a str {
    if amount == 1 { singular } else { plural }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jiff::Zoned;

    #[test]
    fn format_date_spec_relative_days() {
        let cases = [
            ("2026-04-06T16:42:00+00:00[UTC]", "yesterday"),
            ("2026-04-07T16:42:00+00:00[UTC]", "today"),
            ("2026-04-08T16:42:00+00:00[UTC]", "tomorrow"),
        ];

        for (target, expected) in cases {
            assert_eq!(
                strfcur(&Phrase::unresolve(
                    &target.parse::<Zoned>().unwrap(),
                    &"2026-04-07T16:42:00+00:00[UTC]".parse::<Zoned>().unwrap(),
                )),
                expected,
                "target: {target}",
            );
        }
    }

    #[test]
    fn format_date_spec_relative_canonical() {
        let cases = [
            ("2026-03-30T16:42:00+00:00[UTC]", "last Monday"),
            ("2026-03-31T16:42:00+00:00[UTC]", "last Tuesday"),
            ("2026-04-01T16:42:00+00:00[UTC]", "last Wednesday"),
            ("2026-04-02T16:42:00+00:00[UTC]", "last Thursday"),
            ("2026-04-03T16:42:00+00:00[UTC]", "last Friday"),
            ("2026-04-04T16:42:00+00:00[UTC]", "last Saturday"),
            ("2026-04-05T16:42:00+00:00[UTC]", "last Sunday"),
            ("2026-04-06T16:42:00+00:00[UTC]", "yesterday"),
            ("2026-04-07T16:42:00+00:00[UTC]", "today"),
            ("2026-04-08T16:42:00+00:00[UTC]", "tomorrow"),
            ("2026-04-09T16:42:00+00:00[UTC]", "this Thursday"),
            ("2026-04-10T16:42:00+00:00[UTC]", "this Friday"),
            ("2026-04-11T16:42:00+00:00[UTC]", "this Saturday"),
            ("2026-04-12T16:42:00+00:00[UTC]", "this Sunday"),
            ("2026-04-13T16:42:00+00:00[UTC]", "next Monday"),
            ("2026-04-14T16:42:00+00:00[UTC]", "next Tuesday"),
            ("2026-04-15T16:42:00+00:00[UTC]", "next Wednesday"),
            ("2026-04-16T16:42:00+00:00[UTC]", "next Thursday"),
            ("2026-04-17T16:42:00+00:00[UTC]", "next Friday"),
            ("2026-04-18T16:42:00+00:00[UTC]", "next Saturday"),
            ("2026-04-19T16:42:00+00:00[UTC]", "next Sunday"),
        ];

        for (target, expected) in cases {
            assert_eq!(
                strfcur(&Phrase::unresolve(
                    &target.parse::<Zoned>().unwrap(),
                    &"2026-04-07T16:42:00+00:00[UTC]".parse::<Zoned>().unwrap(),
                )),
                expected,
                "target: {target}",
            );
        }
    }

    #[test]
    fn format_date_spec_absolute() {
        let cases = [
            (
                "2024-02-29T09:30:45+00:00[UTC]",
                "on 29 Feb 2024 at 09:30:45",
            ),
            ("2026-04-20T09:00:00+00:00[UTC]", "on 20 Apr 2026 at 09"),
            ("2030-12-31T09:30:00+00:00[UTC]", "on 31 Dec 2030 at 09:30"),
        ];

        for (target, expected) in cases {
            assert_eq!(
                strfcur(&Phrase::unresolve(
                    &target.parse::<Zoned>().unwrap(),
                    &"2026-04-07T16:42:00+00:00[UTC]".parse::<Zoned>().unwrap(),
                )),
                expected,
                "target: {target}",
            );
        }
    }

    #[test]
    fn format_time_spec_today() {
        let cases = [
            ("2026-04-07T00:00:00+00:00[UTC]", "today at 00"),
            ("2026-04-07T09:00:00+00:00[UTC]", "today at 09"),
            ("2026-04-07T09:30:00+00:00[UTC]", "today at 09:30"),
            ("2026-04-07T09:30:45+00:00[UTC]", "today at 09:30:45"),
        ];

        for (target, expected) in cases {
            assert_eq!(
                strfcur(&Phrase::unresolve(
                    &target.parse::<Zoned>().unwrap(),
                    &"2026-04-07T16:42:00+00:00[UTC]".parse::<Zoned>().unwrap(),
                )),
                expected,
                "target: {target}",
            );
        }
    }

    #[test]
    fn format_datetime_spec() {
        let cases = [
            ("2026-04-08T09:00:00+00:00[UTC]", "tomorrow at 09"),
            ("2026-04-10T09:30:00+00:00[UTC]", "this Friday at 09:30"),
            ("2026-04-13T21:30:45+00:00[UTC]", "next Monday at 21:30:45"),
            (
                "2026-05-21T09:30:45+00:00[UTC]",
                "on 21 May 2026 at 09:30:45",
            ),
            ("2026-05-21T16:30:00+00:00[UTC]", "on 21 May 2026 at 16:30"),
            ("2026-04-21T09:30:00+00:00[UTC]", "on 21 Apr 2026 at 09:30"),
        ];

        for (target, expected) in cases {
            assert_eq!(
                strfcur(&Phrase::unresolve(
                    &target.parse::<Zoned>().unwrap(),
                    &"2026-04-07T16:42:00+00:00[UTC]".parse::<Zoned>().unwrap(),
                )),
                expected,
                "target: {target}",
            );
        }
    }

    #[test]
    fn format_calendar_offset_d() {
        let cases = [
            ("2026-04-20T16:42:00+00:00[UTC]", "in 13 days"),
            ("2026-03-25T16:42:00+00:00[UTC]", "13 days ago"),
        ];

        for (target, expected) in cases {
            assert_eq!(
                strfcur(&Phrase::unresolve(
                    &target.parse::<Zoned>().unwrap(),
                    &"2026-04-07T16:42:00+00:00[UTC]".parse::<Zoned>().unwrap(),
                )),
                expected,
                "target: {target}",
            );
        }
    }

    #[test]
    fn format_calendar_offset_w() {
        let cases = [
            ("2026-04-21T16:42:00+00:00[UTC]", "in 2 weeks"),
            ("2026-03-24T16:42:00+00:00[UTC]", "2 weeks ago"),
        ];

        for (target, expected) in cases {
            assert_eq!(
                strfcur(&Phrase::unresolve(
                    &target.parse::<Zoned>().unwrap(),
                    &"2026-04-07T16:42:00+00:00[UTC]".parse::<Zoned>().unwrap(),
                )),
                expected,
                "target: {target}",
            );
        }
    }

    #[test]
    fn format_calendar_offset_m() {
        let cases = [
            (
                "2026-05-07T16:42:00+00:00[UTC]",
                "2026-04-07T16:42:00+00:00[UTC]",
                "in 1 month",
            ),
            (
                "2026-03-07T16:42:00+00:00[UTC]",
                "2026-04-07T16:42:00+00:00[UTC]",
                "1 month ago",
            ),
            (
                "2026-06-07T16:42:00+00:00[UTC]",
                "2026-04-07T16:42:00+00:00[UTC]",
                "in 2 months",
            ),
            (
                "2026-02-07T16:42:00+00:00[UTC]",
                "2026-04-07T16:42:00+00:00[UTC]",
                "2 months ago",
            ),
            (
                "2026-02-28T16:42:00+00:00[UTC]",
                "2026-01-31T16:42:00+00:00[UTC]",
                "in 1 month",
            ),
            (
                "2026-02-28T16:42:00+00:00[UTC]",
                "2026-03-31T16:42:00+00:00[UTC]",
                "1 month ago",
            ),
        ];

        for (target, reference, expected) in cases {
            assert_eq!(
                strfcur(&Phrase::unresolve(
                    &target.parse::<Zoned>().unwrap(),
                    &reference.parse::<Zoned>().unwrap(),
                )),
                expected,
                "target: {target}; reference: {reference}",
            );
        }
    }

    #[test]
    fn format_calendar_offset_y() {
        let cases = [
            (
                "2027-04-07T16:42:00+00:00[UTC]",
                "2026-04-07T16:42:00+00:00[UTC]",
                "in 1 year",
            ),
            (
                "2025-04-07T16:42:00+00:00[UTC]",
                "2026-04-07T16:42:00+00:00[UTC]",
                "1 year ago",
            ),
            (
                "2028-04-07T16:42:00+00:00[UTC]",
                "2026-04-07T16:42:00+00:00[UTC]",
                "in 2 years",
            ),
            (
                "2024-04-07T16:42:00+00:00[UTC]",
                "2026-04-07T16:42:00+00:00[UTC]",
                "2 years ago",
            ),
            (
                "2025-02-28T16:42:00+00:00[UTC]",
                "2024-02-29T16:42:00+00:00[UTC]",
                "in 1 year",
            ),
            (
                "2023-02-28T16:42:00+00:00[UTC]",
                "2024-02-29T16:42:00+00:00[UTC]",
                "1 year ago",
            ),
        ];

        for (target, reference, expected) in cases {
            assert_eq!(
                strfcur(&Phrase::unresolve(
                    &target.parse::<Zoned>().unwrap(),
                    &reference.parse::<Zoned>().unwrap(),
                )),
                expected,
                "target: {target}; reference: {reference}",
            );
        }
    }
}
