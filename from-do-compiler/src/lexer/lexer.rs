use std::sync::LazyLock;

use crate::lexer::block_lexer::{BlockToken, SBlockToken};

use super::{Span, Spannable};
use regex::Regex;

/// A token represents some lines of the input.
/// Except for EOF, all tokens have the content they represent.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Token {
    /// A placeholder for an erroneous token.
    Error(String),
    /// EOF.
    EOF,

    /// Multiple newline characters.
    Line(String),
    /// Multiple space characters (excluding newline characters).
    Space(String),

    /// The colon ':' in a directive block.
    DirectiveHead(String),
    /// The identifiers in a directive block.
    DirectiveArg(String),

    /// The dash '-' in a to-do block.
    ToDoHead(String),
    /// The tab '\t' in a to-do block.
    ToDoIndent(String),
    /// The content of a to-do block, excluding the dash and tab prefixes.
    ToDoContent(String),
}

pub type SToken = Spannable<Token>;

#[derive(Clone)]
pub struct Lexer<Iter>
where
    Iter: Iterator<Item = SBlockToken>,
{
    source: Iter,
    eof: bool,
}

impl<Iter> Lexer<Iter>
where
    Iter: Iterator<Item = SBlockToken>,
{
    pub fn new(input: Iter) -> Self {
        Self {
            source: input,
            eof: false,
        }
    }

    fn token(node: Token, lo: usize, hi: usize) -> SToken {
        Spannable {
            node,
            span: Span { lo, hi },
        }
    }

    fn block_error(value: String, span: Span) -> Vec<SToken> {
        vec![Self::token(Token::Error(value), span.lo, span.hi)]
    }
}

fn next_match(str: &str, regex: &Regex) -> Option<(String, usize, usize)> {
    let mat = regex.find(str)?;

    let lo = mat.start();
    let hi = mat.end();
    let value = str[lo..hi].to_string();

    Some((value, lo, hi))
}

const SPACE_REGEX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[^\S\n]+").unwrap());

const DIRECTIVE_HEAD_REGEX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^:").unwrap());
const DIRECTIVE_ARG_REGEX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\S+").unwrap());

const TODO_HEAD_REGEX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^-").unwrap());
const TODO_INDENT_REGEX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\t").unwrap());
const TODO_CONTENT_REGEX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[^\n]+").unwrap());

impl<Iter> Iterator for Lexer<Iter>
where
    Iter: Iterator<Item = SBlockToken>,
{
    type Item = Vec<SToken>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.eof {
            return None;
        }

        let Spannable { node: block, span } = self.source.next()?;

        match block {
            BlockToken::EOF => {
                self.eof = true;
                Some(vec![Self::token(Token::EOF, span.lo, span.hi)])
            }
            BlockToken::Error(v) => Some(vec![Self::token(Token::Error(v), span.lo, span.hi)]),

            BlockToken::Separation(v) => Some(vec![Self::token(Token::Line(v), span.lo, span.hi)]),

            BlockToken::Directive(v) => {
                let mut args = Vec::new();

                let Some((head_str, lo, hi)) = next_match(&v, &DIRECTIVE_HEAD_REGEX) else {
                    return Some(Self::block_error(v, span));
                };
                let head = Self::token(Token::DirectiveHead(head_str), span.lo + lo, span.lo + hi);
                args.push(head);
                let mut curr = span.lo + hi;
                let mut rest: &str = &v[hi..];

                let Some((arg_str, lo, hi)) = next_match(&rest, &DIRECTIVE_ARG_REGEX) else {
                    return Some(Self::block_error(v, span));
                };
                let arg = Self::token(Token::DirectiveArg(arg_str), curr + lo, curr + hi);
                args.push(arg);
                curr += hi;
                rest = &rest[hi..];

                loop {
                    if let Some((space_str, lo, hi)) = next_match(&rest, &SPACE_REGEX) {
                        let space = Self::token(Token::Space(space_str), curr + lo, curr + hi);
                        args.push(space);
                        curr += hi;
                        rest = &rest[hi..];
                    } else {
                        break;
                    }

                    if let Some((arg_str, lo, hi)) = next_match(&rest, &DIRECTIVE_ARG_REGEX) {
                        let arg = Self::token(Token::DirectiveArg(arg_str), curr + lo, curr + hi);
                        args.push(arg);
                        curr += hi;
                        rest = &rest[hi..];
                    } else {
                        break;
                    }
                }

                if rest.len() > 0 {
                    panic!(
                        "Unexpected characters in directive block: '{}' in '{}' at position {}",
                        &rest, &v, curr
                    );
                }
                Some(args)
            }

            BlockToken::ToDoHeader(v) => {
                let mut args = Vec::new();

                let Some((head_str, lo, hi)) = next_match(&v, &TODO_HEAD_REGEX) else {
                    return Some(Self::block_error(v, span));
                };
                let head = Self::token(Token::ToDoHead(head_str), span.lo + lo, span.lo + hi);
                args.push(head);
                let mut curr = span.lo + hi;
                let mut rest: &str = &v[hi..];

                let Some((indent_str, lo, hi)) = next_match(&rest, &TODO_INDENT_REGEX) else {
                    return Some(Self::block_error(v, span));
                };
                let indent = Self::token(Token::ToDoIndent(indent_str), curr + lo, curr + hi);
                args.push(indent);
                curr += hi;
                rest = &rest[hi..];

                if let Some((content_str, lo, hi)) = next_match(&rest, &TODO_CONTENT_REGEX) {
                    let content =
                        Self::token(Token::ToDoContent(content_str), curr + lo, curr + hi);
                    args.push(content);
                    curr += hi;
                    rest = &rest[hi..];
                }

                if rest.len() > 0 {
                    panic!(
                        "Unexpected characters in todo header block: '{}' in '{}' at position {}",
                        &rest, &v, curr
                    );
                }

                Some(args)
            }

            BlockToken::ToDoContinuation(v) => {
                let mut args = Vec::new();
                let mut curr = span.lo;
                let mut rest: &str = &v;

                let Some((indent_str, lo, hi)) = next_match(&rest, &TODO_INDENT_REGEX) else {
                    return Some(Self::block_error(v, span));
                };
                let indent = Self::token(Token::ToDoIndent(indent_str), curr + lo, curr + hi);
                args.push(indent);
                curr += hi;
                rest = &rest[hi..];

                if let Some((content_str, lo, hi)) = next_match(&rest, &TODO_CONTENT_REGEX) {
                    let content =
                        Self::token(Token::ToDoContent(content_str), curr + lo, curr + hi);
                    args.push(content);
                    curr += hi;
                    rest = &rest[hi..];
                }

                if rest.len() > 0 {
                    panic!(
                        "Unexpected characters in todo continuation block: '{}' in '{}' at position {}",
                        &rest, &v, curr
                    );
                }

                Some(args)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::block_lexer::BlockToken;
    use crate::lexer::{Span, Spannable};

    fn auto_span_bt(bt: impl Iterator<Item = BlockToken>) -> impl Iterator<Item = SBlockToken> {
        let mut current = 0;
        bt.map(move |token| {
            let len = match &token {
                BlockToken::EOF => 0,
                BlockToken::Error(v) => v.len(),
                BlockToken::Separation(v) => v.len(),
                BlockToken::Directive(v) => v.len(),
                BlockToken::ToDoHeader(v) => v.len(),
                BlockToken::ToDoContinuation(v) => v.len(),
            };
            let span = Span {
                lo: current,
                hi: current + len,
            };
            current += len;
            Spannable { node: token, span }
        })
    }

    fn assert(
        input: Vec<BlockToken>,
        expected_nodes: Vec<Token>,
        expected_spans: Vec<(usize, usize)>,
    ) {
        let tokens = Lexer::new(auto_span_bt(input.into_iter()))
            .flat_map(|ts| ts)
            .collect::<Vec<_>>();
        let nodes = tokens.iter().map(|t| t.node.clone()).collect::<Vec<_>>();
        let spans = tokens
            .iter()
            .map(|t| (t.span.lo, t.span.hi))
            .collect::<Vec<_>>();

        assert_eq!(nodes, expected_nodes);
        assert_eq!(spans, expected_spans);
    }

    #[test]
    fn sanity() {
        assert(
            vec![
                BlockToken::Directive(":now 2026-04-08T08:00:00Z".to_string()),
                BlockToken::Separation("\n\n".to_string()),
                BlockToken::ToDoHeader("-\tHello, FromDo! due 2026-04-08T08:00:00Z".to_string()),
                BlockToken::Separation("\n".to_string()),
                BlockToken::EOF,
            ],
            vec![
                Token::DirectiveHead(":".to_string()),
                Token::DirectiveArg("now".to_string()),
                Token::Space(" ".to_string()),
                Token::DirectiveArg("2026-04-08T08:00:00Z".to_string()),
                Token::Line("\n\n".to_string()),
                Token::ToDoHead("-".to_string()),
                Token::ToDoIndent("\t".to_string()),
                Token::ToDoContent("Hello, FromDo! due 2026-04-08T08:00:00Z".to_string()),
                Token::Line("\n".to_string()),
                Token::EOF,
            ],
            vec![
                (0, 1),
                (1, 4),
                (4, 5),
                (5, 25),
                (25, 27),
                (27, 28),
                (28, 29),
                (29, 68),
                (68, 69),
                (69, 69),
            ],
        );
    }

    #[test]
    fn directive_1() {
        assert(
            vec![BlockToken::Directive(":test".to_string())],
            vec![
                Token::DirectiveHead(":".to_string()),
                Token::DirectiveArg("test".to_string()),
            ],
            vec![(0, 1), (1, 5)],
        );
    }

    #[test]
    fn directive_3() {
        assert(
            vec![BlockToken::Directive(":test xx yy zz".to_string())],
            vec![
                Token::DirectiveHead(":".to_string()),
                Token::DirectiveArg("test".to_string()),
                Token::Space(" ".to_string()),
                Token::DirectiveArg("xx".to_string()),
                Token::Space(" ".to_string()),
                Token::DirectiveArg("yy".to_string()),
                Token::Space(" ".to_string()),
                Token::DirectiveArg("zz".to_string()),
            ],
            vec![
                (0, 1),
                (1, 5),
                (5, 6),
                (6, 8),
                (8, 9),
                (9, 11),
                (11, 12),
                (12, 14),
            ],
        );
    }

    #[test]
    fn directive_partial() {
        assert(
            vec![BlockToken::Directive(":test xx yy ".to_string())],
            vec![
                Token::DirectiveHead(":".to_string()),
                Token::DirectiveArg("test".to_string()),
                Token::Space(" ".to_string()),
                Token::DirectiveArg("xx".to_string()),
                Token::Space(" ".to_string()),
                Token::DirectiveArg("yy".to_string()),
                Token::Space(" ".to_string()),
            ],
            vec![(0, 1), (1, 5), (5, 6), (6, 8), (8, 9), (9, 11), (11, 12)],
        );
    }

    #[test]
    fn directive_error() {
        assert(
            vec![BlockToken::Directive(": test".to_string()), BlockToken::EOF],
            vec![Token::Error(": test".to_string()), Token::EOF],
            vec![(0, 6), (6, 6)],
        );
    }

    #[test]
    fn directive_error_continue() {
        assert(
            vec![
                BlockToken::Directive(": test".to_string()),
                BlockToken::Directive(":test".to_string()),
            ],
            vec![
                Token::Error(": test".to_string()),
                Token::DirectiveHead(":".to_string()),
                Token::DirectiveArg("test".to_string()),
            ],
            vec![(0, 6), (6, 7), (7, 11)],
        );
    }

    #[test]
    fn todo_header_simple() {
        assert(
            vec![BlockToken::ToDoHeader("-\tFromDo".to_string())],
            vec![
                Token::ToDoHead("-".to_string()),
                Token::ToDoIndent("\t".to_string()),
                Token::ToDoContent("FromDo".to_string()),
            ],
            vec![(0, 1), (1, 2), (2, 8)],
        );
    }

    #[test]
    fn todo_header_complex() {
        assert(
            vec![BlockToken::ToDoHeader(
                "-\tHello, FromDo! due 2026-04-08T08:00:00Z".to_string(),
            )],
            vec![
                Token::ToDoHead("-".to_string()),
                Token::ToDoIndent("\t".to_string()),
                Token::ToDoContent("Hello, FromDo! due 2026-04-08T08:00:00Z".to_string()),
            ],
            vec![(0, 1), (1, 2), (2, 41)],
        );
    }

    #[test]
    fn todo_header_null() {
        assert(
            vec![BlockToken::ToDoHeader("-\t".to_string())],
            vec![
                Token::ToDoHead("-".to_string()),
                Token::ToDoIndent("\t".to_string()),
            ],
            vec![(0, 1), (1, 2)],
        );
    }

    #[test]
    fn todo_header_error() {
        assert(
            vec![
                BlockToken::ToDoHeader("-FromDo".to_string()),
                BlockToken::EOF,
            ],
            vec![Token::Error("-FromDo".to_string()), Token::EOF],
            vec![(0, 7), (7, 7)],
        );
    }

    #[test]
    fn todo_header_error_continue() {
        assert(
            vec![
                BlockToken::ToDoHeader("-FromDo".to_string()),
                BlockToken::ToDoHeader("-\tFromDo".to_string()),
            ],
            vec![
                Token::Error("-FromDo".to_string()),
                Token::ToDoHead("-".to_string()),
                Token::ToDoIndent("\t".to_string()),
                Token::ToDoContent("FromDo".to_string()),
            ],
            vec![(0, 7), (7, 8), (8, 9), (9, 15)],
        );
    }

    #[test]
    fn todo_continuation_simple() {
        assert(
            vec![BlockToken::ToDoContinuation(
                "\twhat's the buzz?".to_string(),
            )],
            vec![
                Token::ToDoIndent("\t".to_string()),
                Token::ToDoContent("what's the buzz?".to_string()),
            ],
            vec![(0, 1), (1, 17)],
        );
    }

    #[test]
    fn todo_continuation_partial() {
        assert(
            vec![BlockToken::ToDoContinuation("\t ^_^ ".to_string())],
            vec![
                Token::ToDoIndent("\t".to_string()),
                Token::ToDoContent(" ^_^ ".to_string()),
            ],
            vec![(0, 1), (1, 6)],
        );
    }

    #[test]
    fn todo_continuation_error_continue() {
        assert(
            vec![
                BlockToken::ToDoContinuation("FromDo".to_string()),
                BlockToken::ToDoContinuation("\tFromDo".to_string()),
            ],
            vec![
                Token::Error("FromDo".to_string()),
                Token::ToDoIndent("\t".to_string()),
                Token::ToDoContent("FromDo".to_string()),
            ],
            vec![(0, 6), (6, 7), (7, 13)],
        );
    }

    #[test]
    fn todo_continuation_error() {
        assert(
            vec![
                BlockToken::ToDoContinuation("FromDo".to_string()),
                BlockToken::EOF,
            ],
            vec![Token::Error("FromDo".to_string()), Token::EOF],
            vec![(0, 6), (6, 6)],
        );
    }

    #[test]
    fn error() {
        assert(
            vec![BlockToken::Error("what's the buzz?".to_string())],
            vec![Token::Error("what's the buzz?".to_string())],
            vec![(0, 16)],
        );
    }

    #[test]
    fn eof() {
        assert(vec![BlockToken::EOF], vec![Token::EOF], vec![(0, 0)]);
    }
}
