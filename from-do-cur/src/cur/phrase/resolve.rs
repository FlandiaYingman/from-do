use jiff::{
    ToSpan, Zoned,
    civil::{Date, DateTime, Time, Weekday},
};

use super::*;

impl Phrase {
    pub fn resolve(&self, reference: &Zoned) -> Zoned {
        match self {
            Phrase::DateTime { date, time } => {
                let date = date.resolve(reference.date());
                let time = time.resolve(reference.time());
                DateTime::from_parts(date, time)
                    .to_zoned(reference.time_zone().clone())
                    .unwrap()
            }
            Phrase::CalendarOffset(offset) => offset.resolve(reference),
        }
    }
}

impl DateSpec {
    pub(crate) fn resolve(&self, reference_date: Date) -> Date {
        match self {
            Self::Today => reference_date,
            Self::Tomorrow => reference_date.saturating_add(1.day()),
            Self::Yesterday => reference_date.saturating_sub(1.day()),
            Self::Weekday(relation, weekday) => {
                let this_monday = reference_date
                    .saturating_add(1.day())
                    .nth_weekday(-1, Weekday::Monday)
                    .unwrap();
                let that_monday = match relation {
                    Relation::Last => this_monday.saturating_sub(1.week()),
                    Relation::This => this_monday,
                    Relation::Next => this_monday.saturating_add(1.week()),
                };
                that_monday.saturating_add(weekday.to_monday_zero_offset().days())
            }
            Self::Absolute(date) => date.clone(),
        }
    }
}

impl TimeSpec {
    pub(crate) fn resolve(&self, reference_time: Time) -> Time {
        match self {
            Self::Absolute(time) => time.clone(),
            Self::Unspecified => reference_time,
        }
    }
}

impl CalendarOffset {
    pub(crate) fn resolve(&self, r#ref: &Zoned) -> Zoned {
        match self {
            CalendarOffset {
                direction: Direction::Future,
                unit: CalendarUnit::Day,
                amount,
            } => r#ref.saturating_add(amount.days()),
            CalendarOffset {
                direction: Direction::Future,
                unit: CalendarUnit::Week,
                amount,
            } => r#ref.saturating_add(amount.weeks()),
            CalendarOffset {
                direction: Direction::Future,
                unit: CalendarUnit::Month,
                amount,
            } => r#ref.saturating_add(amount.months()),
            CalendarOffset {
                direction: Direction::Future,
                unit: CalendarUnit::Year,
                amount,
            } => r#ref.saturating_add(amount.years()),
            CalendarOffset {
                direction: Direction::Past,
                unit: CalendarUnit::Day,
                amount,
            } => r#ref.saturating_sub(amount.days()),
            CalendarOffset {
                direction: Direction::Past,
                unit: CalendarUnit::Week,
                amount,
            } => r#ref.saturating_sub(amount.weeks()),
            CalendarOffset {
                direction: Direction::Past,
                unit: CalendarUnit::Month,
                amount,
            } => r#ref.saturating_sub(amount.months()),
            CalendarOffset {
                direction: Direction::Past,
                unit: CalendarUnit::Year,
                amount,
            } => r#ref.saturating_sub(amount.years()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_date_spec_yesterday() {
        assert_eq!(
            DateSpec::Yesterday.resolve("2026-04-07".parse::<Date>().unwrap()),
            "2026-04-06".parse::<Date>().unwrap(),
        );
    }

    #[test]
    fn resolve_date_spec_today() {
        assert_eq!(
            DateSpec::Today.resolve("2026-04-07".parse::<Date>().unwrap()),
            "2026-04-07".parse::<Date>().unwrap(),
        );
    }

    #[test]
    fn resolve_date_spec_tomorrow() {
        assert_eq!(
            DateSpec::Tomorrow.resolve("2026-04-07".parse::<Date>().unwrap()),
            "2026-04-08".parse::<Date>().unwrap(),
        );
    }

    #[test]
    fn resolve_date_spec_weekdays() {
        let cases = [
            (
                DateSpec::Weekday(Relation::Last, Weekday::Monday),
                "2026-03-30",
            ),
            (
                DateSpec::Weekday(Relation::Last, Weekday::Tuesday),
                "2026-03-31",
            ),
            (
                DateSpec::Weekday(Relation::Last, Weekday::Wednesday),
                "2026-04-01",
            ),
            (
                DateSpec::Weekday(Relation::Last, Weekday::Thursday),
                "2026-04-02",
            ),
            (
                DateSpec::Weekday(Relation::Last, Weekday::Friday),
                "2026-04-03",
            ),
            (
                DateSpec::Weekday(Relation::Last, Weekday::Saturday),
                "2026-04-04",
            ),
            (
                DateSpec::Weekday(Relation::Last, Weekday::Sunday),
                "2026-04-05",
            ),
            (
                DateSpec::Weekday(Relation::This, Weekday::Monday),
                "2026-04-06",
            ),
            (
                DateSpec::Weekday(Relation::This, Weekday::Tuesday),
                "2026-04-07",
            ),
            (
                DateSpec::Weekday(Relation::This, Weekday::Wednesday),
                "2026-04-08",
            ),
            (
                DateSpec::Weekday(Relation::This, Weekday::Thursday),
                "2026-04-09",
            ),
            (
                DateSpec::Weekday(Relation::This, Weekday::Friday),
                "2026-04-10",
            ),
            (
                DateSpec::Weekday(Relation::This, Weekday::Saturday),
                "2026-04-11",
            ),
            (
                DateSpec::Weekday(Relation::This, Weekday::Sunday),
                "2026-04-12",
            ),
            (
                DateSpec::Weekday(Relation::Next, Weekday::Monday),
                "2026-04-13",
            ),
            (
                DateSpec::Weekday(Relation::Next, Weekday::Tuesday),
                "2026-04-14",
            ),
            (
                DateSpec::Weekday(Relation::Next, Weekday::Wednesday),
                "2026-04-15",
            ),
            (
                DateSpec::Weekday(Relation::Next, Weekday::Thursday),
                "2026-04-16",
            ),
            (
                DateSpec::Weekday(Relation::Next, Weekday::Friday),
                "2026-04-17",
            ),
            (
                DateSpec::Weekday(Relation::Next, Weekday::Saturday),
                "2026-04-18",
            ),
            (
                DateSpec::Weekday(Relation::Next, Weekday::Sunday),
                "2026-04-19",
            ),
        ];

        for (spec, expected) in cases {
            assert_eq!(
                spec.resolve("2026-04-07".parse::<Date>().unwrap()),
                expected.parse::<Date>().unwrap(),
            );
        }
    }

    #[test]
    fn resolve_date_spec_absolute() {
        let cases = ["2024-02-29", "2026-04-20", "2030-12-31"];

        for value in cases {
            let value = value.parse::<Date>().unwrap();

            assert_eq!(
                DateSpec::Absolute(value.clone()).resolve("2026-04-07".parse::<Date>().unwrap()),
                value,
            );
        }
    }

    #[test]
    fn resolve_time_spec_unspecified() {
        assert_eq!(
            TimeSpec::Unspecified.resolve("16:42:00".parse::<Time>().unwrap()),
            "16:42:00".parse::<Time>().unwrap(),
        );
    }

    #[test]
    fn resolve_time_spec_absolute() {
        let cases = ["00:00:00", "09:00:00", "09:30:00", "21:30:45"];

        for value in cases {
            let value = value.parse::<Time>().unwrap();

            assert_eq!(
                TimeSpec::Absolute(value.clone()).resolve("16:42:00".parse::<Time>().unwrap()),
                value,
            );
        }
    }

    #[test]
    fn resolve_datetime_spec() {
        let cases = [
            (
                Phrase::DateTime {
                    date: DateSpec::Yesterday,
                    time: TimeSpec::Unspecified,
                },
                "2026-04-06T16:42:00+00:00[UTC]",
            ),
            (
                Phrase::DateTime {
                    date: DateSpec::Today,
                    time: TimeSpec::Unspecified,
                },
                "2026-04-07T16:42:00+00:00[UTC]",
            ),
            (
                Phrase::DateTime {
                    date: DateSpec::Tomorrow,
                    time: TimeSpec::Unspecified,
                },
                "2026-04-08T16:42:00+00:00[UTC]",
            ),
            (
                Phrase::DateTime {
                    date: DateSpec::Today,
                    time: TimeSpec::Absolute("00:00:00".parse::<Time>().unwrap()),
                },
                "2026-04-07T00:00:00+00:00[UTC]",
            ),
            (
                Phrase::DateTime {
                    date: DateSpec::Today,
                    time: TimeSpec::Absolute("09:00:00".parse::<Time>().unwrap()),
                },
                "2026-04-07T09:00:00+00:00[UTC]",
            ),
            (
                Phrase::DateTime {
                    date: DateSpec::Today,
                    time: TimeSpec::Absolute("09:30:00".parse::<Time>().unwrap()),
                },
                "2026-04-07T09:30:00+00:00[UTC]",
            ),
            (
                Phrase::DateTime {
                    date: DateSpec::Weekday(Relation::This, Weekday::Friday),
                    time: TimeSpec::Absolute("09:30:45".parse::<Time>().unwrap()),
                },
                "2026-04-10T09:30:45+00:00[UTC]",
            ),
            (
                Phrase::DateTime {
                    date: DateSpec::Absolute("2026-12-25".parse::<Date>().unwrap()),
                    time: TimeSpec::Absolute("21:30:45".parse::<Time>().unwrap()),
                },
                "2026-12-25T21:30:45+00:00[UTC]",
            ),
        ];

        for (phrase, expected) in cases {
            let expected = expected.parse::<Zoned>().unwrap();

            assert_eq!(
                phrase.resolve(&"2026-04-07T16:42:00+00:00[UTC]".parse::<Zoned>().unwrap()),
                expected,
            );
        }
    }

    #[test]
    fn resolve_calendar_offset_d() {
        let cases = [
            (
                CalendarOffset {
                    amount: 3,
                    direction: Direction::Future,
                    unit: CalendarUnit::Day,
                },
                "2026-04-10T16:42:00+00:00[UTC]",
            ),
            (
                CalendarOffset {
                    amount: 3,
                    direction: Direction::Past,
                    unit: CalendarUnit::Day,
                },
                "2026-04-04T16:42:00+00:00[UTC]",
            ),
        ];

        for (calendar_offset, expected) in cases {
            let expected = expected.parse::<Zoned>().unwrap();

            assert_eq!(
                calendar_offset
                    .clone()
                    .resolve(&"2026-04-07T16:42:00+00:00[UTC]".parse::<Zoned>().unwrap()),
                expected,
            );
            assert_eq!(
                Phrase::CalendarOffset(calendar_offset)
                    .resolve(&"2026-04-07T16:42:00+00:00[UTC]".parse::<Zoned>().unwrap()),
                expected,
            );
        }
    }

    #[test]
    fn resolve_calendar_offset_w() {
        let cases = [
            (
                CalendarOffset {
                    amount: 2,
                    direction: Direction::Future,
                    unit: CalendarUnit::Week,
                },
                "2026-04-21T16:42:00+00:00[UTC]",
            ),
            (
                CalendarOffset {
                    amount: 2,
                    direction: Direction::Past,
                    unit: CalendarUnit::Week,
                },
                "2026-03-24T16:42:00+00:00[UTC]",
            ),
        ];

        for (calendar_offset, expected) in cases {
            let expected = expected.parse::<Zoned>().unwrap();

            assert_eq!(
                calendar_offset
                    .clone()
                    .resolve(&"2026-04-07T16:42:00+00:00[UTC]".parse::<Zoned>().unwrap()),
                expected,
            );
            assert_eq!(
                Phrase::CalendarOffset(calendar_offset)
                    .resolve(&"2026-04-07T16:42:00+00:00[UTC]".parse::<Zoned>().unwrap()),
                expected,
            );
        }
    }

    #[test]
    fn resolve_calendar_offset_m() {
        let cases = [
            (
                CalendarOffset {
                    amount: 1,
                    direction: Direction::Future,
                    unit: CalendarUnit::Month,
                },
                "2026-04-07T16:42:00+00:00[UTC]",
                "2026-05-07T16:42:00+00:00[UTC]",
            ),
            (
                CalendarOffset {
                    amount: 1,
                    direction: Direction::Past,
                    unit: CalendarUnit::Month,
                },
                "2026-04-07T16:42:00+00:00[UTC]",
                "2026-03-07T16:42:00+00:00[UTC]",
            ),
            (
                CalendarOffset {
                    amount: 1,
                    direction: Direction::Future,
                    unit: CalendarUnit::Month,
                },
                "2026-01-31T16:42:00+00:00[UTC]",
                "2026-02-28T16:42:00+00:00[UTC]",
            ),
            (
                CalendarOffset {
                    amount: 1,
                    direction: Direction::Past,
                    unit: CalendarUnit::Month,
                },
                "2026-03-31T16:42:00+00:00[UTC]",
                "2026-02-28T16:42:00+00:00[UTC]",
            ),
        ];

        for (calendar_offset, reference, expected) in cases {
            let expected = expected.parse::<Zoned>().unwrap();

            assert_eq!(
                calendar_offset
                    .clone()
                    .resolve(&reference.parse::<Zoned>().unwrap()),
                expected,
            );
            assert_eq!(
                Phrase::CalendarOffset(calendar_offset)
                    .resolve(&reference.parse::<Zoned>().unwrap()),
                expected,
            );
        }
    }

    #[test]
    fn resolve_calendar_offset_y() {
        let cases = [
            (
                CalendarOffset {
                    amount: 1,
                    direction: Direction::Future,
                    unit: CalendarUnit::Year,
                },
                "2026-04-07T16:42:00+00:00[UTC]",
                "2027-04-07T16:42:00+00:00[UTC]",
            ),
            (
                CalendarOffset {
                    amount: 1,
                    direction: Direction::Past,
                    unit: CalendarUnit::Year,
                },
                "2026-04-07T16:42:00+00:00[UTC]",
                "2025-04-07T16:42:00+00:00[UTC]",
            ),
            (
                CalendarOffset {
                    amount: 1,
                    direction: Direction::Future,
                    unit: CalendarUnit::Year,
                },
                "2024-02-29T16:42:00+00:00[UTC]",
                "2025-02-28T16:42:00+00:00[UTC]",
            ),
            (
                CalendarOffset {
                    amount: 1,
                    direction: Direction::Past,
                    unit: CalendarUnit::Year,
                },
                "2024-02-29T16:42:00+00:00[UTC]",
                "2023-02-28T16:42:00+00:00[UTC]",
            ),
        ];

        for (calendar_offset, reference, expected) in cases {
            let expected = expected.parse::<Zoned>().unwrap();

            assert_eq!(
                calendar_offset
                    .clone()
                    .resolve(&reference.parse::<Zoned>().unwrap()),
                expected,
            );
            assert_eq!(
                Phrase::CalendarOffset(calendar_offset)
                    .resolve(&reference.parse::<Zoned>().unwrap()),
                expected,
            );
        }
    }
}
