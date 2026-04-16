pub mod block_lexer;
pub mod lexer;

pub use block_lexer::BlockLexer;
pub use block_lexer::BlockToken;

pub use lexer::Lexer;
pub use lexer::Token;

#[derive(Copy, Debug, Clone, PartialEq, Eq)]
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

impl std::ops::AddAssign for Span {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

pub type SString = Spannable<String>;

impl SString {
    pub fn new(node: impl Into<String>, lo: usize, hi: usize) -> Self {
        debug_assert!(lo <= hi);
        Self {
            node: node.into(),
            span: Span { lo, hi },
        }
    }
}

impl From<SString> for String {
    fn from(v: SString) -> String {
        v.node
    }
}

impl std::ops::Add for SString {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            node: self.node + &rhs.node,
            span: self.span + rhs.span,
        }
    }
}

impl std::ops::Add<usize> for SString {
    type Output = Self;

    fn add(self, rhs: usize) -> Self::Output {
        Self {
            node: self.node,
            span: Span {
                lo: self.span.lo + rhs,
                hi: self.span.hi + rhs,
            },
        }
    }
}

impl std::fmt::Display for SString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "'{}' ({}:{})", self.node, self.span.lo, self.span.hi)
    }
}
