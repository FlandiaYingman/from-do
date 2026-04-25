use jiff::{
    RoundMode, ToSpan, Unit, Zoned,
    civil::{Date, DateDifference, Time, Weekday},
};

use super::*;

impl Phrase {
    pub fn unresolve(target: &Zoned, reference: &Zoned) -> Phrase {
        let date_spec = DateSpec::unresolve(&target.date(), &reference.date());
        let time_spec = TimeSpec::unresolve(&target.time(), &reference.time());

        if !matches!(date_spec, DateSpec::Absolute(_)) {
            return Phrase::DateTime {
                date: date_spec,
                time: time_spec,
            };
        }

        let calendar_offset = CalendarOffset::unresolve(target, reference);
        if let Some(offset) = calendar_offset {
            return Phrase::CalendarOffset(offset);
        }

        Phrase::DateTime {
            date: date_spec,
            time: time_spec,
        }
    }
}

impl DateSpec {
    pub(crate) fn unresolve(target: &Date, reference: &Date) -> DateSpec {
        match () {
            () if *target == reference.saturating_sub(1.day()) => return DateSpec::Yesterday,
            () if *target == *reference => return DateSpec::Today,
            () if *target == reference.saturating_add(1.day()) => return DateSpec::Tomorrow,
            _ => {}
        }

        let reference_monday = reference
            .saturating_add(1.day())
            .nth_weekday(-1, Weekday::Monday)
            .unwrap();
        let target_monday = target
            .saturating_add(1.day())
            .nth_weekday(-1, Weekday::Monday)
            .unwrap();
        match () {
            () if target_monday == reference_monday.saturating_sub(1.week()) => {
                return DateSpec::Weekday(Relation::Last, target.weekday());
            }
            () if target_monday == reference_monday => {
                return DateSpec::Weekday(Relation::This, target.weekday());
            }
            () if target_monday == reference_monday.saturating_add(1.week()) => {
                return DateSpec::Weekday(Relation::Next, target.weekday());
            }
            _ => {}
        }

        DateSpec::Absolute(target.clone())
    }
}

impl TimeSpec {
    pub(crate) fn unresolve(target: &Time, reference: &Time) -> TimeSpec {
        if *target == *reference {
            TimeSpec::Unspecified
        } else {
            TimeSpec::Absolute(target.clone())
        }
    }
}

impl CalendarOffset {
    pub(crate) fn unresolve(target: &Zoned, reference: &Zoned) -> Option<CalendarOffset> {
        if target.time() != reference.time() {
            return None;
        }

        let target_date = target.date();
        let reference_date = reference.date();
        let direction = if target_date < reference_date {
            Direction::Past
        } else {
            Direction::Future
        };

        let diff_y = target_date
            .since(
                DateDifference::new(reference_date)
                    .smallest(Unit::Year)
                    .mode(RoundMode::HalfExpand),
            )
            .unwrap();
        if reference_date.saturating_add(diff_y) == target_date
            || reference_date == target_date.saturating_sub(diff_y)
        {
            let amount = diff_y.get_years().abs() as i64;
            return Some(CalendarOffset {
                amount,
                direction,
                unit: CalendarUnit::Year,
            });
        }

        let diff_m = target_date
            .since(
                DateDifference::new(reference_date)
                    .smallest(Unit::Month)
                    .mode(RoundMode::HalfExpand),
            )
            .unwrap();
        if reference_date.saturating_add(diff_m) == target_date
            || reference_date == target_date.saturating_sub(diff_m)
        {
            let amount = diff_m.get_months().abs() as i64;
            return Some(CalendarOffset {
                amount,
                direction,
                unit: CalendarUnit::Month,
            });
        }

        let diff_w = target_date
            .since(
                DateDifference::new(reference_date)
                    .smallest(Unit::Week)
                    .mode(RoundMode::HalfExpand),
            )
            .unwrap();
        if reference_date.saturating_add(diff_w) == target_date
            || reference_date == target_date.saturating_sub(diff_w)
        {
            let amount = diff_w.get_weeks().abs() as i64;
            return Some(CalendarOffset {
                amount,
                direction,
                unit: CalendarUnit::Week,
            });
        }

        let diff_d = target_date
            .since(
                DateDifference::new(reference_date)
                    .smallest(Unit::Day)
                    .mode(RoundMode::HalfExpand),
            )
            .unwrap();
        if reference_date.saturating_add(diff_d) == target_date
            || reference_date == target_date.saturating_sub(diff_d)
        {
            let amount = diff_d.get_days().abs() as i64;
            return Some(CalendarOffset {
                amount,
                direction,
                unit: CalendarUnit::Day,
            });
        }

        unreachable!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unresolve_date_spec_yesterday() {
        assert_eq!(
            DateSpec::unresolve(
                &"2026-04-06".parse::<Date>().unwrap(),
                &"2026-04-07".parse::<Date>().unwrap(),
            ),
            DateSpec::Yesterday,
        );
    }

    #[test]
    fn unresolve_date_spec_today() {
        assert_eq!(
            DateSpec::unresolve(
                &"2026-04-07".parse::<Date>().unwrap(),
                &"2026-04-07".parse::<Date>().unwrap(),
            ),
            DateSpec::Today,
        );
    }

    #[test]
    fn unresolve_date_spec_tomorrow() {
        assert_eq!(
            DateSpec::unresolve(
                &"2026-04-08".parse::<Date>().unwrap(),
                &"2026-04-07".parse::<Date>().unwrap(),
            ),
            DateSpec::Tomorrow,
        );
    }

    #[test]
    fn unresolve_date_spec_weekdays() {
        let cases = [
            (
                "2026-03-30",
                DateSpec::Weekday(Relation::Last, Weekday::Monday),
            ),
            (
                "2026-03-31",
                DateSpec::Weekday(Relation::Last, Weekday::Tuesday),
            ),
            (
                "2026-04-01",
                DateSpec::Weekday(Relation::Last, Weekday::Wednesday),
            ),
            (
                "2026-04-02",
                DateSpec::Weekday(Relation::Last, Weekday::Thursday),
            ),
            (
                "2026-04-03",
                DateSpec::Weekday(Relation::Last, Weekday::Friday),
            ),
            (
                "2026-04-04",
                DateSpec::Weekday(Relation::Last, Weekday::Saturday),
            ),
            (
                "2026-04-05",
                DateSpec::Weekday(Relation::Last, Weekday::Sunday),
            ),
            ("2026-04-06", DateSpec::Yesterday),
            ("2026-04-07", DateSpec::Today),
            ("2026-04-08", DateSpec::Tomorrow),
            (
                "2026-04-09",
                DateSpec::Weekday(Relation::This, Weekday::Thursday),
            ),
            (
                "2026-04-10",
                DateSpec::Weekday(Relation::This, Weekday::Friday),
            ),
            (
                "2026-04-11",
                DateSpec::Weekday(Relation::This, Weekday::Saturday),
            ),
            (
                "2026-04-12",
                DateSpec::Weekday(Relation::This, Weekday::Sunday),
            ),
            (
                "2026-04-13",
                DateSpec::Weekday(Relation::Next, Weekday::Monday),
            ),
            (
                "2026-04-14",
                DateSpec::Weekday(Relation::Next, Weekday::Tuesday),
            ),
            (
                "2026-04-15",
                DateSpec::Weekday(Relation::Next, Weekday::Wednesday),
            ),
            (
                "2026-04-16",
                DateSpec::Weekday(Relation::Next, Weekday::Thursday),
            ),
            (
                "2026-04-17",
                DateSpec::Weekday(Relation::Next, Weekday::Friday),
            ),
            (
                "2026-04-18",
                DateSpec::Weekday(Relation::Next, Weekday::Saturday),
            ),
            (
                "2026-04-19",
                DateSpec::Weekday(Relation::Next, Weekday::Sunday),
            ),
        ];

        for (target, expected) in cases {
            assert_eq!(
                DateSpec::unresolve(
                    &target.parse::<Date>().unwrap(),
                    &"2026-04-07".parse::<Date>().unwrap(),
                ),
                expected,
            );
        }
    }

    #[test]
    fn unresolve_date_spec_absolute() {
        let cases = ["2024-02-29", "2026-04-20", "2030-12-31"];

        for value in cases {
            let value = value.parse::<Date>().unwrap();

            assert_eq!(
                DateSpec::unresolve(&value, &"2026-04-07".parse::<Date>().unwrap()),
                DateSpec::Absolute(value),
            );
        }
    }

    #[test]
    fn unresolve_time_spec_unspecified() {
        assert_eq!(
            TimeSpec::unresolve(
                &"16:42:00".parse::<Time>().unwrap(),
                &"16:42:00".parse::<Time>().unwrap(),
            ),
            TimeSpec::Unspecified,
        );
    }

    #[test]
    fn unresolve_time_spec_absolute() {
        let cases = ["00:00:00", "09:00:00", "09:30:00", "21:30:45"];

        for value in cases {
            let value = value.parse::<Time>().unwrap();

            assert_eq!(
                TimeSpec::unresolve(&value, &"16:42:00".parse::<Time>().unwrap()),
                TimeSpec::Absolute(value),
            );
        }
    }

    #[test]
    fn unresolve_datetime_spec() {
        let cases = [
            (
                "2026-04-06T16:42:00+00:00[UTC]",
                Phrase::DateTime {
                    date: DateSpec::Yesterday,
                    time: TimeSpec::Unspecified,
                },
            ),
            (
                "2026-04-07T16:42:00+00:00[UTC]",
                Phrase::DateTime {
                    date: DateSpec::Today,
                    time: TimeSpec::Unspecified,
                },
            ),
            (
                "2026-04-08T16:42:00+00:00[UTC]",
                Phrase::DateTime {
                    date: DateSpec::Tomorrow,
                    time: TimeSpec::Unspecified,
                },
            ),
            (
                "2026-04-10T16:42:00+00:00[UTC]",
                Phrase::DateTime {
                    date: DateSpec::Weekday(Relation::This, Weekday::Friday),
                    time: TimeSpec::Unspecified,
                },
            ),
            (
                "2026-04-07T00:00:00+00:00[UTC]",
                Phrase::DateTime {
                    date: DateSpec::Today,
                    time: TimeSpec::Absolute("00:00:00".parse::<Time>().unwrap()),
                },
            ),
            (
                "2026-04-07T09:00:00+00:00[UTC]",
                Phrase::DateTime {
                    date: DateSpec::Today,
                    time: TimeSpec::Absolute("09:00:00".parse::<Time>().unwrap()),
                },
            ),
            (
                "2026-04-07T09:30:00+00:00[UTC]",
                Phrase::DateTime {
                    date: DateSpec::Today,
                    time: TimeSpec::Absolute("09:30:00".parse::<Time>().unwrap()),
                },
            ),
            (
                "2026-04-10T09:30:45+00:00[UTC]",
                Phrase::DateTime {
                    date: DateSpec::Weekday(Relation::This, Weekday::Friday),
                    time: TimeSpec::Absolute("09:30:45".parse::<Time>().unwrap()),
                },
            ),
            (
                "2026-05-07T09:30:00+00:00[UTC]",
                Phrase::DateTime {
                    date: DateSpec::Absolute("2026-05-07".parse::<Date>().unwrap()),
                    time: TimeSpec::Absolute("09:30:00".parse::<Time>().unwrap()),
                },
            ),
            (
                "2026-12-25T21:30:45+00:00[UTC]",
                Phrase::DateTime {
                    date: DateSpec::Absolute("2026-12-25".parse::<Date>().unwrap()),
                    time: TimeSpec::Absolute("21:30:45".parse::<Time>().unwrap()),
                },
            ),
        ];

        for (target, expected) in cases {
            assert_eq!(
                Phrase::unresolve(
                    &target.parse::<Zoned>().unwrap(),
                    &"2026-04-07T16:42:00+00:00[UTC]".parse::<Zoned>().unwrap(),
                ),
                expected,
            );
        }
    }

    #[test]
    fn unresolve_calendar_offset_d() {
        let cases = [
            (
                "2026-04-10T16:42:00+00:00[UTC]",
                CalendarOffset {
                    amount: 3,
                    direction: Direction::Future,
                    unit: CalendarUnit::Day,
                },
            ),
            (
                "2026-04-04T16:42:00+00:00[UTC]",
                CalendarOffset {
                    amount: 3,
                    direction: Direction::Past,
                    unit: CalendarUnit::Day,
                },
            ),
        ];

        for (target, expected) in cases {
            assert_eq!(
                CalendarOffset::unresolve(
                    &target.parse::<Zoned>().unwrap(),
                    &"2026-04-07T16:42:00+00:00[UTC]".parse::<Zoned>().unwrap(),
                ),
                Some(expected),
            );
        }
    }

    #[test]
    fn unresolve_calendar_offset_w() {
        let cases = [
            (
                "2026-04-21T16:42:00+00:00[UTC]",
                "2026-04-07T16:42:00+00:00[UTC]",
                CalendarOffset {
                    amount: 2,
                    direction: Direction::Future,
                    unit: CalendarUnit::Week,
                },
            ),
            (
                "2026-03-24T16:42:00+00:00[UTC]",
                "2026-04-07T16:42:00+00:00[UTC]",
                CalendarOffset {
                    amount: 2,
                    direction: Direction::Past,
                    unit: CalendarUnit::Week,
                },
            ),
        ];

        for (target, reference, expected) in cases {
            assert_eq!(
                CalendarOffset::unresolve(
                    &target.parse::<Zoned>().unwrap(),
                    &reference.parse::<Zoned>().unwrap(),
                ),
                Some(expected.clone()),
            );
            assert_eq!(
                Phrase::unresolve(
                    &target.parse::<Zoned>().unwrap(),
                    &reference.parse::<Zoned>().unwrap(),
                ),
                Phrase::CalendarOffset(expected),
            );
        }
    }

    #[test]
    fn unresolve_calendar_offset_m() {
        let cases = [
            (
                "2026-05-07T16:42:00+00:00[UTC]",
                "2026-04-07T16:42:00+00:00[UTC]",
                CalendarOffset {
                    amount: 1,
                    direction: Direction::Future,
                    unit: CalendarUnit::Month,
                },
            ),
            (
                "2026-03-07T16:42:00+00:00[UTC]",
                "2026-04-07T16:42:00+00:00[UTC]",
                CalendarOffset {
                    amount: 1,
                    direction: Direction::Past,
                    unit: CalendarUnit::Month,
                },
            ),
            (
                "2026-02-28T16:42:00+00:00[UTC]",
                "2026-01-31T16:42:00+00:00[UTC]",
                CalendarOffset {
                    amount: 1,
                    direction: Direction::Future,
                    unit: CalendarUnit::Month,
                },
            ),
            (
                "2026-02-28T16:42:00+00:00[UTC]",
                "2026-03-31T16:42:00+00:00[UTC]",
                CalendarOffset {
                    amount: 1,
                    direction: Direction::Past,
                    unit: CalendarUnit::Month,
                },
            ),
        ];

        for (target, reference, expected) in cases {
            assert_eq!(
                CalendarOffset::unresolve(
                    &target.parse::<Zoned>().unwrap(),
                    &reference.parse::<Zoned>().unwrap(),
                ),
                Some(expected.clone()),
            );
            assert_eq!(
                Phrase::unresolve(
                    &target.parse::<Zoned>().unwrap(),
                    &reference.parse::<Zoned>().unwrap(),
                ),
                Phrase::CalendarOffset(expected),
            );
        }
    }

    #[test]
    fn unresolve_calendar_offset_y() {
        let cases = [
            (
                "2027-04-07T16:42:00+00:00[UTC]",
                "2026-04-07T16:42:00+00:00[UTC]",
                CalendarOffset {
                    amount: 1,
                    direction: Direction::Future,
                    unit: CalendarUnit::Year,
                },
            ),
            (
                "2025-04-07T16:42:00+00:00[UTC]",
                "2026-04-07T16:42:00+00:00[UTC]",
                CalendarOffset {
                    amount: 1,
                    direction: Direction::Past,
                    unit: CalendarUnit::Year,
                },
            ),
            (
                "2025-02-28T16:42:00+00:00[UTC]",
                "2024-02-29T16:42:00+00:00[UTC]",
                CalendarOffset {
                    amount: 1,
                    direction: Direction::Future,
                    unit: CalendarUnit::Year,
                },
            ),
            (
                "2023-02-28T16:42:00+00:00[UTC]",
                "2024-02-29T16:42:00+00:00[UTC]",
                CalendarOffset {
                    amount: 1,
                    direction: Direction::Past,
                    unit: CalendarUnit::Year,
                },
            ),
        ];

        for (target, reference, expected) in cases {
            assert_eq!(
                CalendarOffset::unresolve(
                    &target.parse::<Zoned>().unwrap(),
                    &reference.parse::<Zoned>().unwrap(),
                ),
                Some(expected.clone()),
            );
            assert_eq!(
                Phrase::unresolve(
                    &target.parse::<Zoned>().unwrap(),
                    &reference.parse::<Zoned>().unwrap(),
                ),
                Phrase::CalendarOffset(expected),
            );
        }
    }
}
