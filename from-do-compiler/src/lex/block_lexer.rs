use super::*;

use regex::Regex;

/// A BlockToken is a block-level token in the input.
///
/// A BlockToken stores the string and the span of the token in the input. The
/// span is mainly for error reporting.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum BlockToken {
    /// EOF.
    EOF(SString),
    /// A placeholder for an erroneous block during lexing.
    Error(SString),
    /// A newline character, or more.
    Separation(SString),
    /// A line beginning with ':'.
    Directive(SString),
    /// A line beginning with '-\t'.
    ToDoHeader(SString),
    /// A line beginning with '\t'.
    ToDoContinuation(SString),
}

impl BlockToken {
    pub fn str(&self) -> &SString {
        match self {
            Self::EOF(s)
            | Self::Error(s)
            | Self::Separation(s)
            | Self::Directive(s)
            | Self::ToDoHeader(s)
            | Self::ToDoContinuation(s) => s,
        }
    }

    pub fn span(&self) -> Span {
        self.str().span
    }

    pub fn len(&self) -> usize {
        self.str().node.len()
    }
}

mod re {
    use regex::Regex;
    use std::sync::LazyLock as LL;

    pub static ERROR: LL<Regex> = LL::new(|| Regex::new(r"^[^\n]+").unwrap());
    pub static SEPARATION: LL<Regex> = LL::new(|| Regex::new(r"^\n+").unwrap());
    pub static DIRECTIVE: LL<Regex> = LL::new(|| Regex::new(r"^:[^\n]*").unwrap());
    pub static TODO_HEADER: LL<Regex> = LL::new(|| Regex::new(r"^-\t[^\n]*").unwrap());
    pub static TODO_CONTINUATION: LL<Regex> = LL::new(|| Regex::new(r"^\t[^\n]*").unwrap());
}

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

    fn eof(&mut self) -> BlockToken {
        self.eof = true;
        let index = self.source.len();
        BlockToken::EOF(SString::new(String::new(), index, index))
    }

    fn error(&mut self) -> BlockToken {
        if let Some(s) = self.next_match(&re::ERROR) {
            return BlockToken::Error(s);
        }
        panic!("No skippable error token found at index {}", self.current);
    }

    fn peek(&self, n: usize) -> &str {
        let lo = self.current;
        let iter = self.source[lo..].chars().take(n);
        let size = iter.map(|c| c.len_utf8()).sum::<usize>();
        let hi = lo + size;

        &self.source[lo..hi]
    }

    fn next_match(&mut self, regex: &Regex) -> Option<SString> {
        let mat = regex.find(&self.source[self.current..])?;

        let lo = self.current + mat.start();
        let hi = self.current + mat.end();
        let value = self.source[lo..hi].to_string();
        self.current = hi;

        Some(SString::new(value, lo, hi))
    }
}

impl<'a> Iterator for BlockLexer<'a> {
    type Item = BlockToken;

    fn next(&mut self) -> Option<Self::Item> {
        if self.eof {
            return None;
        }

        if self.current >= self.source.len() {
            return Some(self.eof());
        }

        let token = match self.peek(1) {
            "\n" => {
                if let Some(s) = self.next_match(&re::SEPARATION) {
                    BlockToken::Separation(s)
                } else {
                    self.error()
                }
            }
            ":" => {
                if let Some(s) = self.next_match(&re::DIRECTIVE) {
                    BlockToken::Directive(s)
                } else {
                    self.error()
                }
            }
            "-" => {
                if let Some(s) = self.next_match(&re::TODO_HEADER) {
                    BlockToken::ToDoHeader(s)
                } else {
                    self.error()
                }
            }
            "\t" => {
                if let Some(s) = self.next_match(&re::TODO_CONTINUATION) {
                    BlockToken::ToDoContinuation(s)
                } else {
                    self.error()
                }
            }
            _ => self.error(),
        };

        Some(token)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use indoc::indoc;
    use pretty_assertions::assert_eq;

    fn assert_vec_block_token(input: &str, expected: Vec<BlockToken>) {
        assert_eq!(BlockLexer::new(input).collect::<Vec<_>>(), expected);
    }

    #[test]
    fn sanity() {
        let input = indoc! {"
            :now 2026-04-08T08:00:00Z

            -	Hello, FromDo! due 2026-04-08T08:00:00Z
        "};
        assert_vec_block_token(
            input,
            vec![
                BlockToken::Directive(SString::new(":now 2026-04-08T08:00:00Z", 0, 25)),
                BlockToken::Separation(SString::new("\n\n", 25, 27)),
                BlockToken::ToDoHeader(SString::new(
                    "-\tHello, FromDo! due 2026-04-08T08:00:00Z",
                    27,
                    68,
                )),
                BlockToken::Separation(SString::new("\n", 68, 69)),
                BlockToken::EOF(SString::new(String::new(), 69, 69)),
            ],
        );
    }

    #[test]
    fn todo() {
        let input = indoc! {"
            -	Hello, FromDo! due 2026-04-08T08:00:00Z
            	Veni,
            	vidi,
            	vici.
        "};
        assert_vec_block_token(
            input,
            vec![
                BlockToken::ToDoHeader(SString::new(
                    "-\tHello, FromDo! due 2026-04-08T08:00:00Z",
                    0,
                    41,
                )),
                BlockToken::Separation(SString::new("\n", 41, 42)),
                BlockToken::ToDoContinuation(SString::new("\tVeni,", 42, 48)),
                BlockToken::Separation(SString::new("\n", 48, 49)),
                BlockToken::ToDoContinuation(SString::new("\tvidi,", 49, 55)),
                BlockToken::Separation(SString::new("\n", 55, 56)),
                BlockToken::ToDoContinuation(SString::new("\tvici.", 56, 62)),
                BlockToken::Separation(SString::new("\n", 62, 63)),
                BlockToken::EOF(SString::new(String::new(), 63, 63)),
            ],
        );
    }

    #[test]
    fn error() {
        let input = indoc! {"
            what's the buzz?
        "};
        assert_vec_block_token(
            input,
            vec![
                BlockToken::Error(SString::new("what's the buzz?", 0, 16)),
                BlockToken::Separation(SString::new("\n", 16, 17)),
                BlockToken::EOF(SString::new(String::new(), 17, 17)),
            ],
        );
    }

    #[test]
    fn eof() {
        let input = "";
        assert_vec_block_token(
            input,
            vec![BlockToken::EOF(SString::new(String::new(), 0, 0))],
        );
    }
}
