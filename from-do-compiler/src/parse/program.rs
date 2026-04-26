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
    UnknownToDoProp {
        property: SString,
    },
    CurParseError {
        input: SString,
        message: String,
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
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToDo {
    pub head: SString,
    pub body: Option<SString>,

    pub due: Option<property::Due>,
    pub late_due: Option<property::Due>,
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
    }
}
