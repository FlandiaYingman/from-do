use jiff::{
    ToSpan, Zoned,
    civil::{Date, DateTime},
};

use super::pattern::{DayPattern, FieldUnit, Item, MonthPattern, Pattern, YearPattern};

impl Pattern {
    pub fn next(&self, current: &Zoned) -> Option<Zoned> {
        let pattern = self.normalized();
        let mut date = current.date().saturating_add(1.day());

        let matched = loop {
            // advance year (resetting month/day) if current year doesn't match.
            let after_y = pattern.y.next(date)?;
            if after_y != date {
                date = after_y;
                continue;
            }

            // advance month (resetting day, internally rolling year) if current month doesn't match.
            let after_m = pattern.m.next(date)?;
            if after_m != date {
                date = after_m;
                continue;
            }

            // advance day (internally rolling month/year) if current day doesn't match.
            let after_d = pattern.d.next(date)?;
            if after_d != date {
                date = after_d;
                continue;
            }

            break date;
        };

        DateTime::from_parts(matched, current.time())
            .to_zoned(current.time_zone().clone())
            .unwrap()
            .into()
    }
}

impl YearPattern {
    /// Smallest date `>= date` whose year matches, or `None` if none exists
    /// within `1..=9999`. The returned date equals `date` if its year already
    /// matches; otherwise the date is reset to January 1st of the matched year.
    pub(super) fn next(&self, date: Date) -> Option<Date> {
        let from = date.year();
        let y = match self {
            YearPattern::Wildcard => from,
            YearPattern::List(items) => items
                .iter()
                .filter_map(|item| next_in_item(item, FieldUnit::Year, date, from, 9999))
                .min()?,
        };
        if y == date.year() {
            Some(date)
        } else {
            Date::new(y, 1, 1).ok()
        }
    }
}

impl MonthPattern {
    /// Smallest date `>= date` whose month matches. Internally rolls the year
    /// forward when no month in the current year matches. The returned date
    /// equals `date` if its month already matches; otherwise the day is reset
    /// to the 1st.
    pub(super) fn next(&self, date: Date) -> Option<Date> {
        let mut date = date;
        loop {
            let from = i16::from(date.month());
            let candidate = match self {
                MonthPattern::Wildcard => Some(from),
                MonthPattern::List(items) => items
                    .iter()
                    .filter_map(|item| next_in_item(item, FieldUnit::Month, date, from, 12))
                    .min(),
            };
            match candidate {
                Some(m) => {
                    let m = i8::try_from(m).ok()?;
                    return if m == date.month() {
                        Some(date)
                    } else {
                        Date::new(date.year(), m, 1).ok()
                    };
                }
                None => {
                    date = Date::new(date.year() + 1, 1, 1).ok()?;
                }
            }
        }
    }
}

impl DayPattern {
    /// Smallest date `>= date` whose day matches. Internally rolls the month
    /// (and year) forward when no day in the current month matches.
    pub(super) fn next(&self, date: Date) -> Option<Date> {
        let mut date = date;
        loop {
            let last_day = i16::from(date.days_in_month());
            let from = i16::from(date.day());
            let candidate = match self {
                DayPattern::Wildcard => Some(from),
                DayPattern::DayOfMonth(items) => items
                    .iter()
                    .filter_map(|item| {
                        next_in_item(item, FieldUnit::DayOfMonth, date, from, last_day)
                    })
                    .min(),
                DayPattern::DayOfWeek(items) => {
                    let wd_from = i16::from(date.weekday().to_monday_zero_offset()) + 1;
                    Self::next_weekday_offset(items, date, wd_from).and_then(|offset| {
                        let day = from + offset;
                        (day <= last_day).then_some(day)
                    })
                }
            };
            match candidate {
                Some(day) => {
                    let day = i8::try_from(day).ok()?;
                    return if day == date.day() {
                        Some(date)
                    } else {
                        Date::new(date.year(), date.month(), day).ok()
                    };
                }
                None => {
                    let (y, m) = if date.month() == 12 {
                        (date.year() + 1, 1)
                    } else {
                        (date.year(), date.month() + 1)
                    };
                    date = Date::new(y, m, 1).ok()?;
                }
            }
        }
    }

    /// Smallest non-negative offset `k < 7` such that the weekday `wd_from + k`
    /// (mod 7) matches some item, or `None` if no item matches any weekday.
    ///
    /// Split into a non-wrapping pass over `[wd_from, 7]` and a wrapping pass over
    /// `[1, wd_from - 1]` so that [`next_in_item`] can be reused in both passes.
    fn next_weekday_offset(items: &[Item], date: Date, wd_from: i16) -> Option<i16> {
        let direct = items
            .iter()
            .filter_map(|item| next_in_item(item, FieldUnit::DayOfWeek, date, wd_from, 7))
            .min();
        if let Some(wd) = direct {
            return Some(wd - wd_from);
        }
        let wrap = items
            .iter()
            .filter_map(|item| next_in_item(item, FieldUnit::DayOfWeek, date, 1, wd_from - 1))
            .min()?;
        Some(7 - wd_from + wrap)
    }
}

/// Smallest value in `[from, hi]` matching `item` for the given `unit`, or
/// `None`. Negative item bounds are interpreted as counted-from-end values for
/// the unit (resolved against `date` for [`FieldUnit::DayOfMonth`]).
fn next_in_item(item: &Item, unit: FieldUnit, date: Date, from: i16, hi: i16) -> Option<i16> {
    let v = match item {
        Item::Atom(v) => {
            let v = resolve(unit, date, *v);
            if v < from {
                return None;
            }
            v
        }
        Item::Range(s, e, step) => {
            let s = resolve(unit, date, *s);
            let e = resolve(unit, date, *e);
            let step = *step;
            if from <= s {
                s
            } else if from > e {
                return None;
            } else {
                let v = s + ((from - s) + step - 1) / step * step;
                if v > e {
                    return None;
                }
                v
            }
        }
    };
    (v <= hi).then_some(v)
}

/// Resolve a (possibly negative) unit value against a reference date. Negative
/// values count from the unit's last value (e.g. for [`FieldUnit::DayOfMonth`]
/// in February, `-1` resolves to `28` or `29`).
fn resolve(unit: FieldUnit, date: Date, value: i16) -> i16 {
    if value >= 0 {
        return value;
    }
    let last = match unit {
        FieldUnit::Year => 9999,
        FieldUnit::Month => 12,
        FieldUnit::DayOfMonth => i16::from(date.days_in_month()),
        FieldUnit::DayOfWeek => 7,
    };
    last + value + 1
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pat(d: DayPattern, m: MonthPattern, y: YearPattern) -> Pattern {
        Pattern::new(d, m, y)
    }

    #[test]
    fn next_pattern_canonical_cases() {
        let cases = [
            // every-day from a Tue -> Wed.
            (
                pat(
                    DayPattern::Wildcard,
                    MonthPattern::Wildcard,
                    YearPattern::Wildcard,
                ),
                "2026-04-07T16:42:00+00:00[UTC]",
                Some("2026-04-08T16:42:00+00:00[UTC]"),
            ),
            // every Monday from a Tue -> next Monday.
            (
                pat(
                    DayPattern::DayOfWeek(vec![Item::Atom(1)]),
                    MonthPattern::Wildcard,
                    YearPattern::Wildcard,
                ),
                "2026-04-07T16:42:00+00:00[UTC]",
                Some("2026-04-13T16:42:00+00:00[UTC]"),
            ),
            // last day of month: from Jan 30 -> Jan 31.
            (
                pat(
                    DayPattern::DayOfMonth(vec![Item::Atom(-1)]),
                    MonthPattern::Wildcard,
                    YearPattern::Wildcard,
                ),
                "2026-01-30T16:42:00+00:00[UTC]",
                Some("2026-01-31T16:42:00+00:00[UTC]"),
            ),
            // last day of month: from Jan 31 -> Feb 28 (2026 not leap).
            (
                pat(
                    DayPattern::DayOfMonth(vec![Item::Atom(-1)]),
                    MonthPattern::Wildcard,
                    YearPattern::Wildcard,
                ),
                "2026-01-31T16:42:00+00:00[UTC]",
                Some("2026-02-28T16:42:00+00:00[UTC]"),
            ),
            // Feb 29 only in leap years -> next is 2028-02-29.
            (
                pat(
                    DayPattern::DayOfMonth(vec![Item::Atom(29)]),
                    MonthPattern::List(vec![Item::Atom(2)]),
                    YearPattern::Wildcard,
                ),
                "2026-03-01T16:42:00+00:00[UTC]",
                Some("2028-02-29T16:42:00+00:00[UTC]"),
            ),
            // Year-bounded pattern with current past final occurrence -> None.
            (
                pat(
                    DayPattern::Wildcard,
                    MonthPattern::Wildcard,
                    YearPattern::List(vec![Item::Atom(2026)]),
                ),
                "2026-12-31T16:42:00+00:00[UTC]",
                None,
            ),
        ];

        for (pattern, current, expected) in cases {
            let current = current.parse::<Zoned>().unwrap();
            let actual = pattern.next(&current);
            match expected {
                Some(expected) => assert_eq!(
                    actual.unwrap(),
                    expected.parse::<Zoned>().unwrap(),
                    "pattern: {pattern:?}"
                ),
                None => assert!(actual.is_none(), "pattern: {pattern:?}"),
            }
        }
    }

    #[test]
    fn next_pattern_stepped_month_day_range() {
        // Every other day in 1-7 (i.e., 1st, 3rd, 5th, 7th).
        let pattern = pat(
            DayPattern::DayOfMonth(vec![Item::Range(1, 7, 2)]),
            MonthPattern::Wildcard,
            YearPattern::Wildcard,
        );
        let current = "2026-04-04T16:42:00+00:00[UTC]".parse::<Zoned>().unwrap();

        assert_eq!(
            pattern.next(&current).unwrap(),
            "2026-04-05T16:42:00+00:00[UTC]".parse::<Zoned>().unwrap()
        );
    }

    #[test]
    fn next_pattern_weekday_list() {
        // Every Mon and Fri.
        let pattern = pat(
            DayPattern::DayOfWeek(vec![Item::Atom(1), Item::Atom(5)]),
            MonthPattern::Wildcard,
            YearPattern::Wildcard,
        );
        // 2026-04-07 is Tue -> next Friday is 2026-04-10.
        let current = "2026-04-07T16:42:00+00:00[UTC]".parse::<Zoned>().unwrap();

        assert_eq!(
            pattern.next(&current).unwrap(),
            "2026-04-10T16:42:00+00:00[UTC]".parse::<Zoned>().unwrap()
        );
    }

    #[test]
    fn next_year_pattern_skips_non_matching_years() {
        // Year list constrains to 2030; next from 2026 lands on 2030-01-01.
        let pattern = pat(
            DayPattern::Wildcard,
            MonthPattern::Wildcard,
            YearPattern::List(vec![Item::Atom(2030)]),
        );
        let current = "2026-04-07T16:42:00+00:00[UTC]".parse::<Zoned>().unwrap();

        assert_eq!(
            pattern.next(&current).unwrap(),
            "2030-01-01T16:42:00+00:00[UTC]".parse::<Zoned>().unwrap()
        );
    }

    #[test]
    fn next_month_pattern_skips_non_matching_months() {
        // Month list constrains to Mar (3); next from Jan lands on Mar 1.
        let pattern = pat(
            DayPattern::Wildcard,
            MonthPattern::List(vec![Item::Atom(3)]),
            YearPattern::Wildcard,
        );
        let current = "2026-01-15T00:00:00+00:00[UTC]".parse::<Zoned>().unwrap();

        assert_eq!(
            pattern.next(&current).unwrap(),
            "2026-03-01T00:00:00+00:00[UTC]".parse::<Zoned>().unwrap()
        );
    }

    #[test]
    fn next_weekday_wraps_to_following_week() {
        // Every Mon; from a Sun should land on the very next day (offset 1).
        let pattern = pat(
            DayPattern::DayOfWeek(vec![Item::Atom(1)]),
            MonthPattern::Wildcard,
            YearPattern::Wildcard,
        );
        // 2026-04-05 is Sun -> next Mon is 2026-04-06 (cur + 1 day).
        let current = "2026-04-04T00:00:00+00:00[UTC]".parse::<Zoned>().unwrap();

        assert_eq!(
            pattern.next(&current).unwrap(),
            "2026-04-06T00:00:00+00:00[UTC]".parse::<Zoned>().unwrap()
        );
    }
}
