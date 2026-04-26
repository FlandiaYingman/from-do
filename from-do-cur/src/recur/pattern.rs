#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Item {
    Atom(i16),
    Range(i16, i16, i16),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DayPattern {
    Wildcard,
    DayOfWeek(Vec<Item>),
    DayOfMonth(Vec<Item>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MonthPattern {
    Wildcard,
    List(Vec<Item>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum YearPattern {
    Wildcard,
    List(Vec<Item>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pattern {
    pub d: DayPattern,
    pub m: MonthPattern,
    pub y: YearPattern,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FieldUnit {
    DayOfWeek,
    DayOfMonth,
    Month,
    Year,
}

impl Pattern {
    pub fn new(d: DayPattern, m: MonthPattern, y: YearPattern) -> Self {
        Self { d, m, y }
    }
}
