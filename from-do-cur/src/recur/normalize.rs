use std::cmp::Ordering;

use super::pattern::{DayPattern, FieldUnit, Item, MonthPattern, Pattern, YearPattern};

impl Pattern {
    pub fn normalized(&self) -> Pattern {
        Pattern::new(
            normalize_d(&self.d),
            normalize_m(&self.m),
            normalize_y(&self.y),
        )
    }
}

fn normalize_d(d: &DayPattern) -> DayPattern {
    match d {
        DayPattern::Wildcard => DayPattern::Wildcard,
        DayPattern::DayOfWeek(items) => match normalize_list(items, FieldUnit::DayOfWeek) {
            None => DayPattern::Wildcard,
            Some(items) => DayPattern::DayOfWeek(items),
        },
        DayPattern::DayOfMonth(items) => match normalize_list(items, FieldUnit::DayOfMonth) {
            None => DayPattern::Wildcard,
            Some(items) => DayPattern::DayOfMonth(items),
        },
    }
}

fn normalize_m(m: &MonthPattern) -> MonthPattern {
    let MonthPattern::List(items) = m else {
        return MonthPattern::Wildcard;
    };
    match normalize_list(items, FieldUnit::Month) {
        None => MonthPattern::Wildcard,
        Some(items) => MonthPattern::List(items),
    }
}

fn normalize_y(y: &YearPattern) -> YearPattern {
    let YearPattern::List(items) = y else {
        return YearPattern::Wildcard;
    };
    match normalize_list(items, FieldUnit::Year) {
        None => YearPattern::Wildcard,
        Some(items) => YearPattern::List(items),
    }
}

fn normalize_list(items: &[Item], unit: FieldUnit) -> Option<Vec<Item>> {
    let items = items
        .iter()
        .map(|item| normalize_item(item, unit))
        .collect::<Vec<_>>();

    if items.iter().any(|item| matches!(item, None)) {
        return None;
    }

    let mut items = items.into_iter().flatten().collect::<Vec<_>>();

    items.sort();
    items.dedup();

    Some(items)
}

fn normalize_item(item: &Item, unit: FieldUnit) -> Option<Item> {
    match item {
        item @ Item::Atom(_) => Some(item.clone()),
        item @ Item::Range(lo, hi, _) => {
            if *lo == *hi {
                return Some(Item::Atom(*lo));
            }
            if is_full_range(item, unit) {
                return None;
            }
            Some(item.clone())
        }
    }
}

fn is_full_range(item: &Item, unit: FieldUnit) -> bool {
    let Item::Range(lo, hi, 1) = item else {
        return false;
    };
    if matches!((*lo, *hi), (1, -1)) {
        return true;
    }
    if unit == FieldUnit::DayOfWeek && (*lo == 1 || *lo == -7) && *hi >= 7 {
        return true;
    }
    if unit == FieldUnit::DayOfMonth && (*lo == 0 || *lo == 1) && *hi >= 31 {
        return true;
    }
    if unit == FieldUnit::Month && *lo == 1 && *hi >= 12 {
        return true;
    }
    if unit == FieldUnit::Year && *lo == 1 && *hi >= 9999 {
        return true;
    }
    return false;
}

impl Ord for Item {
    fn cmp(&self, other: &Self) -> Ordering {
        let sk = match self {
            Item::Atom(k) => *k,
            Item::Range(k, _, _) => *k,
        };
        let sk = if sk < 0 { i16::MAX + sk } else { sk };
        let ok = match other {
            Item::Atom(k) => *k,
            Item::Range(k, _, _) => *k,
        };
        let ok = if ok < 0 { i16::MAX + ok } else { ok };
        sk.cmp(&ok)
    }
}

impl PartialOrd for Item {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_full_to_wildcard() {
        let cases = [
            (
                Pattern::new(
                    DayPattern::DayOfWeek(vec![Item::Range(1, 7, 1)]),
                    MonthPattern::List(vec![Item::Range(1, 12, 1)]),
                    YearPattern::List(vec![Item::Range(1, 9999, 1)]),
                ),
                Pattern::new(
                    DayPattern::Wildcard,
                    MonthPattern::Wildcard,
                    YearPattern::Wildcard,
                ),
            ),
            (
                Pattern::new(
                    DayPattern::DayOfMonth(vec![Item::Range(1, 31, 1)]),
                    MonthPattern::Wildcard,
                    YearPattern::Wildcard,
                ),
                Pattern::new(
                    DayPattern::Wildcard,
                    MonthPattern::Wildcard,
                    YearPattern::Wildcard,
                ),
            ),
        ];

        for (input, expected) in cases {
            assert_eq!(input.normalized(), expected);
        }
    }

    #[test]
    fn test_singleton_to_atom() {
        let input = Pattern::new(
            DayPattern::DayOfWeek(vec![Item::Range(3, 3, 1)]),
            MonthPattern::List(vec![Item::Range(5, 5, 7)]),
            YearPattern::Wildcard,
        );
        let expected = Pattern::new(
            DayPattern::DayOfWeek(vec![Item::Atom(3)]),
            MonthPattern::List(vec![Item::Atom(5)]),
            YearPattern::Wildcard,
        );

        assert_eq!(input.normalized(), expected);
    }

    #[test]
    fn test_sort_dedup() {
        let input = Pattern::new(
            DayPattern::DayOfWeek(vec![
                Item::Atom(5),
                Item::Atom(1),
                Item::Atom(5),
                Item::Atom(2),
            ]),
            MonthPattern::List(vec![Item::Atom(12), Item::Atom(1), Item::Atom(12)]),
            YearPattern::Wildcard,
        );
        let expected = Pattern::new(
            DayPattern::DayOfWeek(vec![Item::Atom(1), Item::Atom(2), Item::Atom(5)]),
            MonthPattern::List(vec![Item::Atom(1), Item::Atom(12)]),
            YearPattern::Wildcard,
        );

        assert_eq!(input.normalized(), expected);
    }
}
