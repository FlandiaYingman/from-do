use super::*;

use regex::Regex;
use std::collections::*;

/// A Token is a *token* in the input.
///
/// A Token stores the string and the span of the token in the input. The span
/// is mainly for error reporting.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Token {
    /// EOF.
    EOF(SString),
    /// A placeholder for an erroneous token.
    Error(SString),

    /// Multiple newline characters.
    Line(SString),
    /// Multiple space characters (excluding newline characters).
    Space(SString),

    /// The colon ':' in a directive block.
    DirectiveHead(SString),
    /// The identifiers in a directive block.
    DirectiveArg(SString),

    /// The dash '-' in a to-do block.
    ToDoHead(SString),
    /// The tab '\t' in a to-do block.
    ToDoIndent(SString),
    /// The content of a to-do block, excluding the dash and tab prefixes.
    ToDoContent(SString),
}

impl Token {
    pub fn str(&self) -> &SString {
        match self {
            Self::EOF(s)
            | Self::Error(s)
            | Self::Line(s)
            | Self::Space(s)
            | Self::DirectiveHead(s)
            | Self::DirectiveArg(s)
            | Self::ToDoHead(s)
            | Self::ToDoIndent(s)
            | Self::ToDoContent(s) => s,
        }
    }

    pub fn span(&self) -> Span {
        self.str().span
    }

    pub fn len(&self) -> usize {
        self.str().node.len()
    }
}

#[derive(Clone)]
pub struct Lexer<Iter>
where
    Iter: Iterator<Item = BlockToken>,
{
    source: Iter,
    buf: VecDeque<Token>,
}

impl<Iter> Lexer<Iter>
where
    Iter: Iterator<Item = BlockToken>,
{
    pub fn new(input: Iter) -> Self {
        Self {
            source: input,
            buf: VecDeque::new(),
        }
    }
}

fn next_match(input: &SString, regex: &Regex) -> Option<(SString, SString)> {
    match regex.find(&input.node) {
        Some(mat) => {
            let lo = mat.start();
            let hi = mat.end();
            let value = input.node[lo..hi].to_string();

            let m = SString::new(value, lo, hi) + input.span.lo;
            let r = SString {
                node: input.node[mat.end()..].to_string(),
                span: Span {
                    lo: input.span.lo + hi,
                    hi: input.span.hi,
                },
            };

            Some((m, r))
        }
        None => None,
    }
}

mod re {
    use regex::Regex;
    use std::sync::LazyLock as LL;

    pub static SPACE: LL<Regex> = LL::new(|| Regex::new(r"^[^\S\n]+").unwrap());

    pub static DIRECTIVE_HEAD: LL<Regex> = LL::new(|| Regex::new(r"^:").unwrap());
    pub static DIRECTIVE_ARG: LL<Regex> = LL::new(|| Regex::new(r"^\S+").unwrap());

    pub static TODO_HEAD: LL<Regex> = LL::new(|| Regex::new(r"^-").unwrap());
    pub static TODO_INDENT: LL<Regex> = LL::new(|| Regex::new(r"^\t").unwrap());
    pub static TODO_CONTENT: LL<Regex> = LL::new(|| Regex::new(r"^[^\n]+").unwrap());
}

impl<Iter> Lexer<Iter>
where
    Iter: Iterator<Item = BlockToken>,
{
    fn lex(&mut self) -> Option<Vec<Token>> {
        let token = self.source.next()?;
        let mut tokens = Vec::new();

        macro_rules! must_match {
            ($input:expr, $regex:expr) => {
                match next_match($input, $regex) {
                    Some((m, r)) => (m, r),
                    None => {
                        panic!(
                            "Unexpected string in block: {} in {}. Expected {}.",
                            $input,
                            token.str(),
                            $regex.as_str()
                        )
                    }
                }
            };
        }

        match token {
            BlockToken::EOF(v) => {
                tokens.push(Token::EOF(v));
            }
            BlockToken::Error(v) => {
                tokens.push(Token::Error(v));
            }

            BlockToken::Separation(v) => {
                tokens.push(Token::Line(v));
            }

            BlockToken::Directive(ref v) => {
                let mut rest = v.clone();

                let (head, vv) = must_match!(&rest, &re::DIRECTIVE_HEAD);
                tokens.push(Token::DirectiveHead(head));
                rest = vv;

                loop {
                    if let Some((arg, vv)) = next_match(&rest, &re::DIRECTIVE_ARG) {
                        tokens.push(Token::DirectiveArg(arg));
                        rest = vv;
                        continue;
                    };
                    if let Some((space, vv)) = next_match(&rest, &re::SPACE) {
                        tokens.push(Token::Space(space));
                        rest = vv;
                        continue;
                    };
                    break;
                }

                if rest.node.len() != 0 {
                    panic!(
                        "Unexpected non-null rest in directive block: {} in {}",
                        rest, v,
                    );
                }
            }

            BlockToken::ToDoHeader(ref v) => {
                let mut rest = v.clone();

                let (head, vv) = must_match!(&rest, &re::TODO_HEAD);
                tokens.push(Token::ToDoHead(head));
                rest = vv;

                let (indent, vv) = must_match!(&rest, &re::TODO_INDENT);
                tokens.push(Token::ToDoIndent(indent));
                rest = vv;

                if let Some((content, vv)) = next_match(&rest, &re::TODO_CONTENT) {
                    tokens.push(Token::ToDoContent(content));
                    rest = vv;
                }

                if rest.node.len() != 0 {
                    panic!(
                        "Unexpected non-null rest in todo header block: {} in {}",
                        rest, v
                    );
                }
            }

            BlockToken::ToDoContinuation(ref v) => {
                let mut rest = v.clone();

                let (indent, vv) = must_match!(&rest, &re::TODO_INDENT);
                tokens.push(Token::ToDoIndent(indent));
                rest = vv;

                if let Some((content, vv)) = next_match(&rest, &re::TODO_CONTENT) {
                    tokens.push(Token::ToDoContent(content));
                    rest = vv;
                }

                if rest.node.len() != 0 {
                    panic!(
                        "Unexpected non-null rest in todo continuation block: {} in {}",
                        rest, v
                    );
                }
            }
        }

        Some(tokens)
    }
}

impl<Iter> Iterator for Lexer<Iter>
where
    Iter: Iterator<Item = BlockToken>,
{
    type Item = Token;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(token) = self.buf.pop_front() {
            return Some(token);
        }

        let tokens = self.lex()?;
        self.buf.extend(tokens);
        self.buf.pop_front()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    macro_rules! bt {
        ($bt_name:path, $value:expr) => {
            $bt_name(SString::new($value, 0, 0))
        };
    }

    fn with_span(token: BlockToken, lo: usize, hi: usize) -> BlockToken {
        match token {
            BlockToken::EOF(s) => BlockToken::EOF(SString::new(s.node, lo, hi)),
            BlockToken::Error(s) => BlockToken::Error(SString::new(s.node, lo, hi)),
            BlockToken::Separation(s) => BlockToken::Separation(SString::new(s.node, lo, hi)),
            BlockToken::Directive(s) => BlockToken::Directive(SString::new(s.node, lo, hi)),

            BlockToken::ToDoHeader(s) => {
                BlockToken::ToDoHeader(SString::new(s.node, lo, hi)) //
            }
            BlockToken::ToDoContinuation(s) => {
                BlockToken::ToDoContinuation(SString::new(s.node, lo, hi)) //
            }
        }
    }

    fn auto_span(bt: impl Iterator<Item = BlockToken>) -> impl Iterator<Item = BlockToken> {
        let mut current = 0;
        bt.map(move |token| {
            let len = token.len();
            let token = with_span(token, current, current + len);
            current += len;
            token
        })
    }

    fn assert_vec_token(input: Vec<BlockToken>, expected: Vec<Token>) {
        assert_eq!(
            Lexer::new(auto_span(input.into_iter())).collect::<Vec<_>>(),
            expected
        );
    }

    fn assert_panic(input: Vec<BlockToken>, rest: SString, block: SString, re: &Regex) {
        let r = std::panic::catch_unwind(|| {
            Lexer::new(auto_span(input.into_iter())).collect::<Vec<_>>()
        });

        assert!(r.is_err());
        let err = r.err().unwrap();
        let message = err
            .downcast_ref::<String>()
            .map(|s| s.as_str())
            .unwrap_or("<non-string panic message>");

        assert_eq!(
            message,
            format!(
                "Unexpected string in block: {} in {}. Expected {}.",
                rest,
                block,
                re.as_str()
            )
        );
    }

    #[test]
    fn sanity() {
        assert_vec_token(
            vec![
                bt!(BlockToken::Directive, ":now 2026-04-08T08:00:00Z"),
                bt!(BlockToken::Separation, "\n\n"),
                bt!(
                    BlockToken::ToDoHeader,
                    "-\tHello, FromDo! due 2026-04-08T08:00:00Z"
                ),
                bt!(BlockToken::Separation, "\n"),
                bt!(BlockToken::EOF, ""),
            ],
            vec![
                Token::DirectiveHead(SString::new(":", 0, 1)),
                Token::DirectiveArg(SString::new("now", 1, 4)),
                Token::Space(SString::new(" ", 4, 5)),
                Token::DirectiveArg(SString::new("2026-04-08T08:00:00Z", 5, 25)),
                Token::Line(SString::new("\n\n", 25, 27)),
                Token::ToDoHead(SString::new("-", 27, 28)),
                Token::ToDoIndent(SString::new("\t", 28, 29)),
                Token::ToDoContent(SString::new(
                    "Hello, FromDo! due 2026-04-08T08:00:00Z",
                    29,
                    68,
                )),
                Token::Line(SString::new("\n", 68, 69)),
                Token::EOF(SString::new("", 69, 69)),
            ],
        );
    }

    #[test]
    fn directive_1() {
        assert_vec_token(
            vec![bt!(BlockToken::Directive, ":test")],
            vec![
                Token::DirectiveHead(SString::new(":", 0, 1)),
                Token::DirectiveArg(SString::new("test", 1, 5)),
            ],
        );
    }

    #[test]
    fn directive_3() {
        assert_vec_token(
            vec![bt!(BlockToken::Directive, ":test xx yy zz")],
            vec![
                Token::DirectiveHead(SString::new(":", 0, 1)),
                Token::DirectiveArg(SString::new("test", 1, 5)),
                Token::Space(SString::new(" ", 5, 6)),
                Token::DirectiveArg(SString::new("xx", 6, 8)),
                Token::Space(SString::new(" ", 8, 9)),
                Token::DirectiveArg(SString::new("yy", 9, 11)),
                Token::Space(SString::new(" ", 11, 12)),
                Token::DirectiveArg(SString::new("zz", 12, 14)),
            ],
        );
    }

    #[test]
    fn directive_partial() {
        assert_vec_token(
            vec![bt!(BlockToken::Directive, ": test xx yy ")],
            vec![
                Token::DirectiveHead(SString::new(":", 0, 1)),
                Token::Space(SString::new(" ", 1, 2)),
                Token::DirectiveArg(SString::new("test", 2, 6)),
                Token::Space(SString::new(" ", 6, 7)),
                Token::DirectiveArg(SString::new("xx", 7, 9)),
                Token::Space(SString::new(" ", 9, 10)),
                Token::DirectiveArg(SString::new("yy", 10, 12)),
                Token::Space(SString::new(" ", 12, 13)),
            ],
        );
    }

    #[test]
    fn directive_panic() {
        assert_panic(
            vec![bt!(BlockToken::Directive, "test"), bt!(BlockToken::EOF, "")],
            SString::new("test", 0, 4),
            SString::new("test", 0, 4),
            &re::DIRECTIVE_HEAD,
        );
    }

    #[test]
    fn todo_header_simple() {
        assert_vec_token(
            vec![bt!(BlockToken::ToDoHeader, "-\tFromDo")],
            vec![
                Token::ToDoHead(SString::new("-", 0, 1)),
                Token::ToDoIndent(SString::new("\t", 1, 2)),
                Token::ToDoContent(SString::new("FromDo", 2, 8)),
            ],
        );
    }

    #[test]
    fn todo_header_complex() {
        assert_vec_token(
            vec![bt!(
                BlockToken::ToDoHeader,
                "-\tHello, FromDo! due 2026-04-08T08:00:00Z"
            )],
            vec![
                Token::ToDoHead(SString::new("-", 0, 1)),
                Token::ToDoIndent(SString::new("\t", 1, 2)),
                Token::ToDoContent(SString::new(
                    "Hello, FromDo! due 2026-04-08T08:00:00Z",
                    2,
                    41,
                )),
            ],
        );
    }

    #[test]
    fn todo_header_null() {
        assert_vec_token(
            vec![bt!(BlockToken::ToDoHeader, "-\t")],
            vec![
                Token::ToDoHead(SString::new("-", 0, 1)),
                Token::ToDoIndent(SString::new("\t", 1, 2)),
            ],
        );
    }

    #[test]
    fn todo_header_panic() {
        assert_panic(
            vec![
                bt!(BlockToken::ToDoHeader, "FromDo"),
                bt!(BlockToken::EOF, ""),
            ],
            SString::new("FromDo", 0, 6),
            SString::new("FromDo", 0, 6),
            &re::TODO_HEAD,
        );
        assert_panic(
            vec![
                bt!(BlockToken::ToDoHeader, "-FromDo"),
                bt!(BlockToken::EOF, ""),
            ],
            SString::new("FromDo", 1, 7),
            SString::new("-FromDo", 0, 7),
            &re::TODO_INDENT,
        );
    }

    #[test]
    fn todo_continuation_simple() {
        assert_vec_token(
            vec![bt!(BlockToken::ToDoContinuation, "\twhat's the buzz?")],
            vec![
                Token::ToDoIndent(SString::new("\t", 0, 1)),
                Token::ToDoContent(SString::new("what's the buzz?", 1, 17)),
            ],
        );
    }

    #[test]
    fn todo_continuation_partial() {
        assert_vec_token(
            vec![bt!(BlockToken::ToDoContinuation, "\t ^_^ ")],
            vec![
                Token::ToDoIndent(SString::new("\t", 0, 1)),
                Token::ToDoContent(SString::new(" ^_^ ", 1, 6)),
            ],
        );
    }

    #[test]
    fn todo_continuation_panic() {
        assert_panic(
            vec![
                bt!(BlockToken::ToDoContinuation, "FromDo"),
                bt!(BlockToken::EOF, ""),
            ],
            SString::new("FromDo", 0, 6),
            SString::new("FromDo", 0, 6),
            &re::TODO_INDENT,
        );
    }

    #[test]
    fn error() {
        assert_vec_token(
            vec![bt!(BlockToken::Error, "what's the buzz?")],
            vec![Token::Error(SString::new("what's the buzz?", 0, 16))],
        );
    }

    #[test]
    fn eof() {
        assert_vec_token(
            vec![bt!(BlockToken::EOF, "")],
            vec![Token::EOF(SString::new("", 0, 0))],
        );
    }
}
