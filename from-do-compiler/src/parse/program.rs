use crate::lex::*;
use from_do_cur::cur;
use from_do_cur::recur;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Block {
    Error(Error),

    Directive(Directive),
    ToDo(ToDo),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    LexerError(SString),
    UnexpectedToken {
        unexpected: Token,
        expected: &'static str,
    },
    UnexpectedEOF {
        at: Span,
        expected: &'static str,
    },
    UnknownDirective(SString),
    TimestampParseError {
        timestamp: SString,
        message: String,
    },
    TimeZoneParseError {
        time_zone: SString,
        message: String,
    },
    AheadParseError {
        ahead: SString,
        message: String,
    },
    UnknownToDoProp {
        property: SString,
    },
    CurParseError {
        input: SString,
        message: String,
    },
    UnexpectedToDoProp {
        property: SString,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Program {
    pub blocks: Vec<Block>,
}

impl Program {
    pub fn new() -> Self {
        Self { blocks: Vec::new() }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Directive {
    Now(directive::Now),
    Tz(directive::Tz),
    Ahead(directive::Ahead),
}

pub mod directive {
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct Now {
        pub now: jiff::Zoned,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct Tz {
        pub tz: jiff::tz::TimeZone,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct Ahead {
        pub ahead: u32,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToDo {
    pub t: ToDoType,

    pub head: SString,
    pub body: Option<SString>,

    pub schedule: Schedule,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToDoType {
    /// A task that is yet to be done (the "-" prefixed tasks).
    ToDo,
    /// A task that is already done (the "+" prefixed tasks).
    NotToDo,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Schedule {
    Once {
        due: Option<property::Due>,
        late_due: Option<property::Due>,
    },
    Recurring {
        recurring: property::Recurring,
        begin: Option<property::Due>,
        until: Option<property::Due>,
    },
}

impl Schedule {
    /// A Once schedule that have no due date and late due date.
    pub fn never() -> Self {
        Schedule::Once {
            due: None,
            late_due: None,
        }
    }
}

pub mod property {
    use super::*;

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct Due {
        pub rel: Option<cur::Phrase>,
        pub ts: Option<jiff::Zoned>,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct Recurring {
        pub pattern: recur::Pattern,
        pub ts: Option<jiff::Zoned>,
    }
}
