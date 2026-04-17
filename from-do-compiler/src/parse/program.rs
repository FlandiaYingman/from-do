use crate::lex::*;

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
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Program {
    pub blocks: Vec<Block>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Directive {
    Now(directive::Now),
}

pub mod directive {
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct Now {
        pub now: jiff::Timestamp,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToDo {
    pub head: SString,
    pub body: Option<SString>,

    pub due: Option<jiff::Timestamp>,
}
