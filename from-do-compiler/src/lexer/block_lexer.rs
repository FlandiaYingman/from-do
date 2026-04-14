use std::sync::LazyLock;

use super::{Span, Spannable};
use regex::Regex;

/// A block token represents some lines of the input. 
/// Except for EOF, all block tokens have the content they represent.
#[derive(Debug, PartialEq, Eq)]
pub enum BlockToken {
    /// A placeholder for an erroneous block.
    Error(String),
    /// EOF.
    EOF,
    /// Multiple newline characters.
    Separation(String),
    /// A line beginning with ':'.
    Directive(String),
    /// A line beginning with '-' followed by '\t'.
    ToDoHeader(String),
    /// A line beginning with '\t'.
    ToDoContinuation(String),
}

pub type SBlockToken = Spannable<BlockToken>;

pub struct BlockLexer<'a> {
    source: &'a str,
    current: usize,
    eof: bool,
}

impl<'a> BlockLexer<'a> {
    pub fn new(input: &'a str) -> Self {
        Self {
            source: input,
            current: 0,
            eof: false,
        }
    }

    fn token(node: BlockToken, lo: usize, hi: usize) -> SBlockToken {
        Spannable {
            node,
            span: Span { lo, hi },
        }
    }

    fn eof_token(&mut self) -> SBlockToken {
        self.eof = true;
        let index = self.source.len();
        Self::token(BlockToken::EOF, index, index)
    }

    fn peek_str(&self, n: usize) -> &str {
        let lo = self.current;
        let iter = self.source[lo..].chars().take(n);
        let size = iter.map(|c| c.len_utf8()).sum::<usize>();
        let hi = lo + size;

        &self.source[lo..hi]
    }

    const SEPARATOR_REGEX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\n+").unwrap());
    const DIRECTIVE_REGEX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^:[^\n]*").unwrap());
    const TODO_HEADER_REGEX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^-\t[^\n]*").unwrap());
    const TODO_CONTINUATION_REGEX: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"^\t[^\n]*").unwrap());
    const ERROR_REGEX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[^\n]+").unwrap());

    fn next_match(&mut self, regex: &Regex) -> Option<(String, usize, usize)> {
        let mat = regex.find(&self.source[self.current..])?;

        let lo = self.current + mat.start();
        let hi = self.current + mat.end();
        let value = self.source[lo..hi].to_string();
        self.current = hi;

        Some((value, lo, hi))
    }

    fn error(&mut self) -> SBlockToken {
        if let Some((value, lo, hi)) = self.next_match(&Self::ERROR_REGEX) {
            return Self::token(BlockToken::Error(value), lo, hi);
        }

        let lo = self.current;
        let hi = lo + 1;
        self.current = hi;
        Self::token(BlockToken::Error(self.source[lo..hi].to_string()), lo, hi)
    }
}

impl<'a> Iterator for BlockLexer<'a> {
    type Item = SBlockToken;

    fn next(&mut self) -> Option<Self::Item> {
        if self.eof {
            return None;
        }

        if self.current >= self.source.len() {
            return Some(self.eof_token());
        }

        let t = match self.peek_str(1) {
            "\n" => {
                if let Some((value, lo, hi)) = self.next_match(&Self::SEPARATOR_REGEX) {
                    Self::token(BlockToken::Separation(value), lo, hi)
                } else {
                    self.error()
                }
            }
            ":" => {
                if let Some((value, lo, hi)) = self.next_match(&Self::DIRECTIVE_REGEX) {
                    Self::token(BlockToken::Directive(value), lo, hi)
                } else {
                    self.error()
                }
            }
            "-" => {
                if let Some((value, lo, hi)) = self.next_match(&Self::TODO_HEADER_REGEX) {
                    Self::token(BlockToken::ToDoHeader(value), lo, hi)
                } else {
                    self.error()
                }
            }
            "\t" => {
                if let Some((value, lo, hi)) = self.next_match(&Self::TODO_CONTINUATION_REGEX) {
                    Self::token(BlockToken::ToDoContinuation(value), lo, hi)
                } else {
                    self.error()
                }
            }
            _ => self.error(),
        };

        Some(t)
    }
}

#[cfg(test)]
mod tests {
    use super::{BlockLexer, BlockToken};
    use indoc::indoc;

    #[test]
    fn sanity() {
        let input = indoc! {"
            :now 2026-04-08 08:00:00Z

            -	Hello, FromDo! due 2026-04-08 08:00:00Z
        "};
        let tokens: Vec<_> = BlockLexer::new(input).map(|token| token.node).collect();

        assert_eq!(
            tokens,
            vec![
                BlockToken::Directive(":now 2026-04-08 08:00:00Z".to_string()),
                BlockToken::Separation("\n\n".to_string()),
                BlockToken::ToDoHeader("-\tHello, FromDo! due 2026-04-08 08:00:00Z".to_string()),
                BlockToken::Separation("\n".to_string()),
                BlockToken::EOF,
            ]
        );
    }

    #[test]
    fn sanity_span() {
        let input = indoc! {"
            :now 2026-04-08 08:00:00Z

            -	Hello, FromDo! due 2026-04-08 08:00:00Z
        "};
        let spans: Vec<_> = BlockLexer::new(input)
            .map(|token| (token.span.lo, token.span.hi))
            .collect();

        assert_eq!(spans, vec![(0, 25), (25, 27), (27, 68), (68, 69), (69, 69)]);
    }

    #[test]
    fn todo() {
        let input = indoc! {"
            -	Hello, FromDo! due 2026-04-08 08:00:00Z
            	Veni,
            	vidi,
            	vici.
        "};
        let tokens: Vec<_> = BlockLexer::new(input).map(|token| token.node).collect();

        assert_eq!(
            tokens,
            vec![
                BlockToken::ToDoHeader("-\tHello, FromDo! due 2026-04-08 08:00:00Z".to_string()),
                BlockToken::Separation("\n".to_string()),
                BlockToken::ToDoContinuation("\tVeni,".to_string()),
                BlockToken::Separation("\n".to_string()),
                BlockToken::ToDoContinuation("\tvidi,".to_string()),
                BlockToken::Separation("\n".to_string()),
                BlockToken::ToDoContinuation("\tvici.".to_string()),
                BlockToken::Separation("\n".to_string()),
                BlockToken::EOF,
            ]
        );
    }

    #[test]
    fn todo_span() {
        let input = indoc! {"
            -	Hello, FromDo! due 2026-04-08 08:00:00Z
            	Veni,
            	vidi,
            	vici.
        "};
        let spans: Vec<_> = BlockLexer::new(input)
            .map(|token| (token.span.lo, token.span.hi))
            .collect();

        assert_eq!(
            spans,
            vec![
                (0, 41),
                (41, 42),
                (42, 48),
                (48, 49),
                (49, 55),
                (55, 56),
                (56, 62),
                (62, 63),
                (63, 63)
            ]
        );
    }

    #[test]
    fn error() {
        let input = indoc! {"
            what's the buzz?
        "};
        let tokens: Vec<_> = BlockLexer::new(input).map(|token| token.node).collect();

        assert_eq!(
            tokens,
            vec![
                BlockToken::Error("what's the buzz?".to_string()),
                BlockToken::Separation("\n".to_string()),
                BlockToken::EOF
            ]
        );
    }

    #[test]
    fn error_span() {
        let input = indoc! {"
            what's the buzz?
        "};
        let spans: Vec<_> = BlockLexer::new(input)
            .map(|token| (token.span.lo, token.span.hi))
            .collect();

        assert_eq!(spans, vec![(0, 16), (16, 17), (17, 17)]);
    }

    #[test]
    fn eof() {
        let input = "";
        let tokens: Vec<_> = BlockLexer::new(input).map(|token| token.node).collect();

        assert_eq!(tokens, vec![BlockToken::EOF]);
    }

    #[test]
    fn eof_span() {
        let input = "";
        let spans: Vec<_> = BlockLexer::new(input)
            .map(|token| (token.span.lo, token.span.hi))
            .collect();

        assert_eq!(spans, vec![(0, 0)]);
    }
}
