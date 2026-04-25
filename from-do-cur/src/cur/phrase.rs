use jiff::civil::Weekday;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Relation {
    Last,
    This,
    Next,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Direction {
    Past,
    Future,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DateSpec {
    Today,
    Tomorrow,
    Yesterday,
    Weekday(Relation, Weekday),
    Absolute(jiff::civil::Date),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TimeSpec {
    Absolute(jiff::civil::Time),
    Unspecified,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CalendarUnit {
    Day,
    Week,
    Month,
    Year,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CalendarOffset {
    pub amount: i64,
    pub direction: Direction,
    pub unit: CalendarUnit,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Phrase {
    DateTime { date: DateSpec, time: TimeSpec },
    CalendarOffset(CalendarOffset),
}

mod resolve;
mod unresolve;
