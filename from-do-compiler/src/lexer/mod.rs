pub mod block_lexer;
pub mod lexer;

pub use block_lexer::BlockLexer;
pub use lexer::Lexer;

pub struct Span {
    pub lo: usize,
    pub hi: usize,
}

pub struct Spannable<T> {
    pub node: T,
    pub span: Span,
}