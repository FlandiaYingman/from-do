pub mod block_lexer;
pub mod lexer;

#[rustfmt::skip]
mod imports {
    pub use super::block_lexer::BlockLexer;
    pub use super::block_lexer::BlockToken;
    pub use super::block_lexer::SBlockToken;
    pub use super::lexer::Lexer;
    pub use super::lexer::Token;
    pub use super::lexer::SToken;
}
pub use imports::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Span {
    pub lo: usize,
    pub hi: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Spannable<T> {
    pub node: T,
    pub span: Span,
}

impl std::ops::Add for Span {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        let lhs = self;
        Self {
            lo: std::cmp::min(lhs.lo, rhs.lo),
            hi: std::cmp::max(lhs.hi, rhs.hi),
        }
    }
}

pub type SString = Spannable<String>;

impl std::ops::Add for SString {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            node: self.node + &rhs.node,
            span: self.span + rhs.span,
        }
    }
}

impl Into<String> for SString {
    fn into(self) -> String {
        self.node
    }
}
