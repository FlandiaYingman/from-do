pub mod block_lexer;

pub use block_lexer::BlockLexer;

pub struct Span {
	pub lo: usize,
	pub hi: usize,
}

pub struct Spannable<T> {
	pub node: T,
	pub span: Span,
}