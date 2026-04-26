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
                for (offset, line) in v.node.char_indices() {
                    tokens.push(Token::Line(
                        SString::new(line.to_string(), offset, offset + line.len_utf8())
                            + v.span.lo,
                    ));
                }
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

                while let Some((indent, vv)) = next_match(&rest, &re::TODO_INDENT) {
                    tokens.push(Token::ToDoIndent(indent));
                    rest = vv;
                }

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

    #[test]
    fn sanity() {
        //| :now 2026-04-08T08:00:00Z
        //|
        //| -	Hello, FromDo! due 2026-04-08T08:00:00Z
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
                Token::Line(SString::new("\n", 25, 26)),
                Token::Line(SString::new("\n", 26, 27)),
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
        //| :FromDo
        assert_vec_token(
            vec![bt!(BlockToken::Directive, ":FromDo")],
            vec![
                Token::DirectiveHead(SString::new(":", 0, 1)),
                Token::DirectiveArg(SString::new("FromDo", 1, 7)),
            ],
        );
    }

    #[test]
    fn directive_tz() {
        //| :tz America/New_York
        assert_vec_token(
            vec![bt!(BlockToken::Directive, ":tz America/New_York")],
            vec![
                Token::DirectiveHead(SString::new(":", 0, 1)),
                Token::DirectiveArg(SString::new("tz", 1, 3)),
                Token::Space(SString::new(" ", 3, 4)),
                Token::DirectiveArg(SString::new("America/New_York", 4, 20)),
            ],
        );
    }

    #[test]
    fn directive_null() {
        //| :
        assert_vec_token(
            vec![bt!(BlockToken::Directive, ":")],
            vec![Token::DirectiveHead(SString::new(":", 0, 1))],
        );
    }

    #[test]
    fn directive_3() {
        //| :FromDo buzz now
        assert_vec_token(
            vec![bt!(BlockToken::Directive, ":FromDo buzz now")],
            vec![
                Token::DirectiveHead(SString::new(":", 0, 1)),
                Token::DirectiveArg(SString::new("FromDo", 1, 7)),
                Token::Space(SString::new(" ", 7, 8)),
                Token::DirectiveArg(SString::new("buzz", 8, 12)),
                Token::Space(SString::new(" ", 12, 13)),
                Token::DirectiveArg(SString::new("now", 13, 16)),
            ],
        );
    }

    #[test]
    fn directive_partial() {
        //| : FromDo buzz
        assert_vec_token(
            vec![bt!(BlockToken::Directive, ": FromDo buzz ")],
            vec![
                Token::DirectiveHead(SString::new(":", 0, 1)),
                Token::Space(SString::new(" ", 1, 2)),
                Token::DirectiveArg(SString::new("FromDo", 2, 8)),
                Token::Space(SString::new(" ", 8, 9)),
                Token::DirectiveArg(SString::new("buzz", 9, 13)),
                Token::Space(SString::new(" ", 13, 14)),
            ],
        );
    }

    #[test]
    fn todo_header_simple() {
        //| -	FromDo
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
        //| -	Hello, FromDo! due 2026-04-08T08:00:00Z
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
        //| -
        assert_vec_token(
            vec![bt!(BlockToken::ToDoHeader, "-\t")],
            vec![
                Token::ToDoHead(SString::new("-", 0, 1)),
                Token::ToDoIndent(SString::new("\t", 1, 2)),
            ],
        );
    }

    #[test]
    fn todo_continuation_null() {
        //|
        assert_vec_token(
            vec![bt!(BlockToken::ToDoContinuation, "\t")],
            vec![Token::ToDoIndent(SString::new("\t", 0, 1))],
        );
    }

    #[test]
    fn todo_continuation_simple() {
        //| 	What's the buzz?
        assert_vec_token(
            vec![bt!(BlockToken::ToDoContinuation, "\tWhat's the buzz?")],
            vec![
                Token::ToDoIndent(SString::new("\t", 0, 1)),
                Token::ToDoContent(SString::new("What's the buzz?", 1, 17)),
            ],
        );
    }

    #[test]
    fn todo_continuation_partial() {
        //| 	 Hello, FromDo!
        assert_vec_token(
            vec![bt!(BlockToken::ToDoContinuation, "\t Hello, FromDo! ")],
            vec![
                Token::ToDoIndent(SString::new("\t", 0, 1)),
                Token::ToDoContent(SString::new(" Hello, FromDo! ", 1, 17)),
            ],
        );
    }

    #[test]
    fn error() {
        //| What's the buzz?
        assert_vec_token(
            vec![bt!(BlockToken::Error, "What's the buzz?")],
            vec![Token::Error(SString::new("What's the buzz?", 0, 16))],
        );
    }

    #[test]
    fn eof() {
        // empty input
        assert_vec_token(
            vec![bt!(BlockToken::EOF, "")],
            vec![Token::EOF(SString::new("", 0, 0))],
        );
    }
}
