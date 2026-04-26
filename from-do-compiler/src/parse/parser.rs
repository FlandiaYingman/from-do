use super::*;
use crate::lex::*;

use from_do_cur::cur;
use from_do_cur::recur;

use Error::*;
use jiff::tz::TimeZone;

pub struct Parser {
    source: Vec<Token>,
    cursor: usize,
    progress: Span,
}

type Result<T> = std::result::Result<T, Error>;

mod re {
    use super::*;

    use regex::Regex;
    use std::sync::LazyLock as LL;

    static TODO_PROP_HEAD: LL<Regex> = LL::new(|| Regex::new(r"^(due|late due) ?(.*)$").unwrap());

    pub fn todo_prop_head(head: &SString) -> Option<(SString, SString)> {
        TODO_PROP_HEAD
            .captures(&head.node)
            .map(|caps| (caps.get(1).unwrap(), caps.get(2).unwrap()))
            .map(|(m1, m2)| {
                (
                    SString {
                        node: m1.as_str().to_string(),
                        span: Span {
                            lo: head.span.lo + m1.start(),
                            hi: head.span.lo + m1.end(),
                        },
                    },
                    SString {
                        node: m2.as_str().to_string(),
                        span: Span {
                            lo: head.span.lo + m2.start(),
                            hi: head.span.lo + m2.end(),
                        },
                    },
                )
            })
    }
}

impl Parser {
    pub fn new<Iter>(input: Iter) -> Self
    where
        Iter: IntoIterator<Item = Token>,
    {
        Self {
            source: input.into_iter().collect(),
            cursor: 0,
            progress: Span { lo: 0, hi: 0 },
        }
    }

    /// Snapshot the current parser position so it can be restored later via
    /// [`Self::restore`]. The returned value is opaque and only valid for the
    /// parser instance that produced it.
    fn checkpoint(&self) -> (usize, Span) {
        (self.cursor, self.progress)
    }

    /// Restore a previously taken [`Self::checkpoint`], rewinding the cursor
    /// and progress span. Tokens consumed after the checkpoint will be
    /// re-yielded by subsequent calls to [`Self::next`] / [`Self::peek`].
    fn restore(&mut self, checkpoint: (usize, Span)) {
        let (cursor, progress) = checkpoint;
        self.cursor = cursor;
        self.progress = progress;
    }

    fn peek(&self) -> Option<&Token> {
        self.source.get(self.cursor)
    }

    fn next(&mut self, expected: &'static str) -> Result<Token> {
        match self.source.get(self.cursor).cloned() {
            Some(token) => {
                self.cursor += 1;
                self.progress += token.span();
                Ok(token)
            }
            None => Err(UnexpectedEOF {
                at: Span {
                    lo: self.progress.hi,
                    hi: self.progress.hi,
                },
                expected: expected,
            }),
        }
    }

    pub fn program(&mut self) -> Program {
        let mut blocks = Vec::new();

        while let Some(node) = self.peek() {
            if let Token::Line(_) = node {
                self.cursor += 1;
                continue;
            }
            if let Token::EOF(_) = node {
                self.cursor += 1;
                break;
            }
            blocks.push(self.block());
        }

        Program { blocks }
    }

    const BLOCK_EXPECTED: &'static str = "block (head): directive head, to-do head, (and error)";
    fn block(&mut self) -> Block {
        let next = match self.next(Self::BLOCK_EXPECTED) {
            Ok(token) => token,
            Err(e) => return Block::Error(e),
        };
        match next {
            Token::Error(raw) => Block::Error(LexerError(raw)),
            token @ Token::EOF(_) => Block::Error(UnexpectedToken {
                unexpected: token,
                expected: Self::BLOCK_EXPECTED,
            }),

            token @ Token::Line(_) => Block::Error(UnexpectedToken {
                unexpected: token,
                expected: Self::BLOCK_EXPECTED,
            }),
            token @ Token::Space(_) => Block::Error(UnexpectedToken {
                unexpected: token,
                expected: Self::BLOCK_EXPECTED,
            }),

            Token::DirectiveHead(_) => self.directive().map_or_else(Block::Error, Block::Directive),
            token @ Token::DirectiveArg(_) => Block::Error(UnexpectedToken {
                unexpected: token,
                expected: Self::BLOCK_EXPECTED,
            }),

            Token::ToDoHead(_) => self.todo().map_or_else(Block::Error, Block::ToDo),
            token @ Token::ToDoIndent(_) => Block::Error(UnexpectedToken {
                unexpected: token,
                expected: Self::BLOCK_EXPECTED,
            }),
            token @ Token::ToDoContent(_) => Block::Error(UnexpectedToken {
                unexpected: token,
                expected: Self::BLOCK_EXPECTED,
            }),
        }
    }

    const LINE_EXPECTED: &'static str = "line";
    fn line(&mut self) -> Result<SString> {
        match self.next(Self::LINE_EXPECTED)? {
            Token::Line(s) => Ok(s),
            token => Err(Error::UnexpectedToken {
                unexpected: token,
                expected: Self::LINE_EXPECTED,
            }),
        }
    }

    /// Consume a [`Token::Line`] or accept an upcoming [`Token::EOF`] (without
    /// consuming it, so the surrounding [`Self::program`] loop still observes
    /// the EOF and terminates). Returns an empty [`SString`] in the EOF case.
    fn line_or_eof(&mut self) -> Result<SString> {
        if let Some(Token::EOF(s)) = self.peek() {
            return Ok(SString {
                node: String::new(),
                span: Span {
                    lo: s.span.hi,
                    hi: s.span.hi,
                },
            });
        }
        self.line()
    }

    const SPACE_EXPECTED: &'static str = "space";
    fn space(&mut self) -> Result<Token> {
        match self.next(Self::SPACE_EXPECTED)? {
            Token::Space(s) => Ok(Token::Space(s)),
            token => Err(Error::UnexpectedToken {
                unexpected: token,
                expected: Self::SPACE_EXPECTED,
            }),
        }
    }

    fn directive(&mut self) -> Result<Directive> {
        let name = self.directive_arg()?;
        match name.node.as_str() {
            "now" => {
                let _ = self.space()?;
                let now_str = self.directive_arg()?;
                let now = Self::timestamp(&now_str)?;
                Ok(Directive::Now(directive::Now { now }))
            }
            "tz" => {
                let _ = self.space()?;
                let tz_str = self.directive_arg()?;
                let tz = Self::time_zone(&tz_str)?;
                Ok(Directive::Tz(directive::Tz { tz }))
            }
            _ => Err(Error::UnknownDirective(name)),
        }
    }

    const DIRECTIVE_ARG_EXPECTED: &'static str = "directive argument";
    fn directive_arg(&mut self) -> Result<SString> {
        match self.next(Self::DIRECTIVE_ARG_EXPECTED)? {
            Token::DirectiveArg(s) => Ok(s),
            token => Err(Error::UnexpectedToken {
                unexpected: token,
                expected: Self::DIRECTIVE_ARG_EXPECTED,
            }),
        }
    }

    fn todo(&mut self) -> Result<ToDo> {
        let _ = self.todo_indent()?;
        let head = self.todo_content()?;
        let _ = self.line_or_eof()?;

        let mut due = None;
        let mut late_due = None;
        // let mut recurring = None;
        while let Some(Token::ToDoIndent(_)) = self.peek() {
            let _ = self.todo_indent().unwrap();
            if let Some(Token::Line(_)) = self.peek() {
                let _ = self.line().unwrap();
                break;
            }
            let (d, ld, _) = self.todo_prop()?;
            due = d.or(due);
            late_due = ld.or(late_due);
        }

        let mut body_vec = Vec::new();
        'a: while let Some(Token::ToDoIndent(_)) = self.peek() {
            let _ = self.todo_indent().unwrap();
            'b: loop {
                if let Some(Token::Line(_)) = self.peek() {
                    let line = self.line().unwrap();
                    body_vec.push(line);
                    continue 'a;
                } else {
                    break 'b;
                }
            }
            let content = self.todo_content()?;
            body_vec.push(content);
            let line = self.line_or_eof()?;
            body_vec.push(line);
        }
        let body = if body_vec.is_empty() {
            None
        } else {
            Some(body_vec.into_iter().reduce(|c1, c2| c1 + c2).unwrap())
        };

        Ok(ToDo {
            head,
            body,
            due,
            late_due,
        })
    }

    fn todo_prop(
        &mut self,
    ) -> Result<(
        Option<property::Due>,       // due
        Option<property::Due>,       // late due
        Option<property::Recurring>, // recurring
    )> {
        let mut due = None;
        let mut late_due = None;
        let mut recurring = None;

        let prop_content = self.todo_content()?;
        let (prop_head, prop_arg_head) = match re::todo_prop_head(&prop_content) {
            Some((head, arg)) => (head, arg),
            None => {
                return Err(Error::UnknownToDoProp {
                    property: prop_content,
                });
            }
        };
        let _ = self.line_or_eof()?;

        match prop_head.node.as_str() {
            "due" => {
                let prop_rel_str = prop_arg_head;
                let prop_rel = (!prop_rel_str.node.is_empty())
                    .then(|| Self::cur(&prop_rel_str))
                    .transpose()?;

                let prop_ts = self.todo_prop_ts()?;

                due = Some(property::Due {
                    rel: prop_rel,
                    ts: prop_ts,
                });
            }
            "late due" => {
                let prop_rel_str = prop_arg_head;
                let prop_rel = (!prop_rel_str.node.is_empty())
                    .then(|| Self::cur(&prop_rel_str))
                    .transpose()?;

                let prop_ts = self.todo_prop_ts()?;

                late_due = Some(property::Due {
                    rel: prop_rel,
                    ts: prop_ts,
                });
            }
            "recurring" => {
                let prop_pattern_str = prop_arg_head;
                let prop_pattern = Self::recur(&prop_pattern_str)?;
                let _ = self.line_or_eof()?;

                recurring = Some(property::Recurring {
                    pattern: prop_pattern,
                });
            }
            _ => panic!(
                "unreachable: re::todo_prop_head should only capture 'due', 'late due', or 'recurring'"
            ),
        }

        Ok((due, late_due, recurring))
    }

    /// Try to consume the optional `\t\t<timestamp>\n` line that may follow a
    /// `due`/`late due` property head. If the next two tokens are not
    /// `ToDoIndent`, the cursor is restored and `None` is returned.
    fn todo_prop_ts(&mut self) -> Result<Option<jiff::Zoned>> {
        let cp = self.checkpoint();
        let first_is_indent = matches!(self.peek(), Some(Token::ToDoIndent(_)));
        if !first_is_indent {
            return Ok(None);
        }
        let _ = self.todo_indent().unwrap();
        let second_is_indent = matches!(self.peek(), Some(Token::ToDoIndent(_)));
        if !second_is_indent {
            self.restore(cp);
            return Ok(None);
        }
        let _ = self.todo_indent().unwrap();
        let prop_ts_str = self.todo_content()?;
        let prop_ts = (!prop_ts_str.node.is_empty())
            .then(|| Self::timestamp(&prop_ts_str))
            .transpose()?;
        let _ = self.line_or_eof()?;
        Ok(prop_ts)
    }

    const TODO_INDENT_EXPECTED: &'static str = "todo indent";
    fn todo_indent(&mut self) -> Result<SString> {
        match self.next(Self::TODO_INDENT_EXPECTED)? {
            Token::ToDoIndent(s) => Ok(s),
            token => Err(Error::UnexpectedToken {
                unexpected: token,
                expected: Self::TODO_INDENT_EXPECTED,
            }),
        }
    }

    const TODO_CONTENT_EXPECTED: &'static str = "todo content";
    fn todo_content(&mut self) -> Result<SString> {
        match self.next(Self::TODO_CONTENT_EXPECTED)? {
            Token::ToDoContent(s) => Ok(s),
            token => Err(Error::UnexpectedToken {
                unexpected: token,
                expected: Self::TODO_CONTENT_EXPECTED,
            }),
        }
    }
}

impl Parser {
    fn timestamp(timestamp_str: &SString) -> Result<jiff::Zoned> {
        timestamp_str
            .node
            .parse::<jiff::Zoned>()
            .map_err(|err| TimestampParseError {
                timestamp: timestamp_str.clone(),
                message: err.to_string(),
            })
    }

    fn time_zone(tz_str: &SString) -> Result<jiff::tz::TimeZone> {
        TimeZone::get(&tz_str.node).map_err(|err| TimeZoneParseError {
            time_zone: tz_str.clone(),
            message: err.to_string(),
        })
    }

    fn cur(cur_str: &SString) -> Result<cur::Phrase> {
        cur::strpcur(&cur_str.node.clone()).map_err(|err| Error::CurParseError {
            input: cur_str.clone(),
            message: err.to_string(),
        })
    }

    fn recur(recur_str: &SString) -> Result<recur::Pattern> {
        recur::strprecur(&recur_str.node.clone()).map_err(|err| Error::CurParseError {
            input: recur_str.clone(),
            message: err.to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::Parser as _Parser;
    use super::*;
    use indoc::indoc;
    use pretty_assertions::assert_eq;

    macro_rules! t {
        ($bt_name:path, $value:expr) => {
            $bt_name(SString::new($value, 0, 0))
        };
    }

    fn ts(value: &str) -> jiff::Zoned {
        value.parse().unwrap()
    }

    fn tz(name: &str) -> TimeZone {
        TimeZone::get(name).unwrap()
    }

    fn with_span(token: Token, lo: usize, hi: usize) -> Token {
        match token {
            Token::EOF(s) => Token::EOF(SString::new(s.node, lo, hi)),
            Token::Error(s) => Token::Error(SString::new(s.node, lo, hi)),
            Token::Line(s) => Token::Line(SString::new(s.node, lo, hi)),
            Token::Space(s) => Token::Space(SString::new(s.node, lo, hi)),
            Token::DirectiveHead(s) => Token::DirectiveHead(SString::new(s.node, lo, hi)),
            Token::DirectiveArg(s) => Token::DirectiveArg(SString::new(s.node, lo, hi)),
            Token::ToDoHead(s) => Token::ToDoHead(SString::new(s.node, lo, hi)),
            Token::ToDoIndent(s) => Token::ToDoIndent(SString::new(s.node, lo, hi)),
            Token::ToDoContent(s) => Token::ToDoContent(SString::new(s.node, lo, hi)),
        }
    }

    fn auto_span(bt: impl Iterator<Item = Token>) -> impl Iterator<Item = Token> {
        let mut current = 0;
        bt.map(move |token| {
            let len = token.len();
            let token = with_span(token, current, current + len);
            current += len;
            token
        })
    }

    fn assert_program(input: Vec<Token>, expected: Program) {
        assert_eq!(
            Parser::new(auto_span(input.into_iter()).collect::<Vec<_>>().into_iter()).program(),
            expected,
        );
    }

    type Parser = _Parser;

    #[test]
    fn sanity_0() {
        // empty input
        assert_program(vec![t!(Token::EOF, "")], Program { blocks: vec![] });
    }

    #[test]
    fn sanity_1() {
        //| :now 2026-04-08T08:00:00+00:00[UTC]
        //|
        //| -	Hello, FromDo!
        //| 	due
        //| 		2026-04-08T08:00:00+00:00[UTC]
        assert_program(
            vec![
                t!(Token::DirectiveHead, ":"),
                t!(Token::DirectiveArg, "now"),
                t!(Token::Space, " "),
                t!(Token::DirectiveArg, "2026-04-08T08:00:00+00:00[UTC]"),
                t!(Token::Line, "\n"),
                t!(Token::Line, "\n"),
                t!(Token::ToDoHead, "-"),
                t!(Token::ToDoIndent, "\t"),
                t!(Token::ToDoContent, "Hello, FromDo!"),
                t!(Token::Line, "\n"),
                t!(Token::ToDoIndent, "\t"),
                t!(Token::ToDoContent, "due"),
                t!(Token::Line, "\n"),
                t!(Token::ToDoIndent, "\t"),
                t!(Token::ToDoIndent, "\t"),
                t!(Token::ToDoContent, "2026-04-08T08:00:00+00:00[UTC]"),
                t!(Token::Line, "\n"),
                t!(Token::EOF, ""),
            ],
            Program {
                blocks: vec![
                    Block::Directive(Directive::Now(directive::Now {
                        now: ts("2026-04-08T08:00:00+00:00[UTC]"),
                    })),
                    Block::ToDo(ToDo {
                        head: SString {
                            node: "Hello, FromDo!".to_string(),
                            span: Span { lo: 39, hi: 53 },
                        },
                        body: None,
                        due: Some(property::Due {
                            rel: None,
                            ts: Some(ts("2026-04-08T08:00:00+00:00[UTC]")),
                        }),
                        late_due: None,
                    }),
                ],
            },
        );
    }

    #[test]
    fn sanity_2() {
        //| -	Hello, FromDo!
        //| 	due
        //| 		2026-04-08T08:00:00+00:00[UTC]
        //| 	late due
        //| 		2026-04-09T08:00:00+00:00[UTC]
        assert_program(
            vec![
                t!(Token::ToDoHead, "-"),
                t!(Token::ToDoIndent, "\t"),
                t!(Token::ToDoContent, "Hello, FromDo!"),
                t!(Token::Line, "\n"),
                t!(Token::ToDoIndent, "\t"),
                t!(Token::ToDoContent, "due"),
                t!(Token::Line, "\n"),
                t!(Token::ToDoIndent, "\t"),
                t!(Token::ToDoIndent, "\t"),
                t!(Token::ToDoContent, "2026-04-08T08:00:00+00:00[UTC]"),
                t!(Token::Line, "\n"),
                t!(Token::ToDoIndent, "\t"),
                t!(Token::ToDoContent, "late due"),
                t!(Token::Line, "\n"),
                t!(Token::ToDoIndent, "\t"),
                t!(Token::ToDoIndent, "\t"),
                t!(Token::ToDoContent, "2026-04-09T08:00:00+00:00[UTC]"),
                t!(Token::Line, "\n"),
                t!(Token::EOF, ""),
            ],
            Program {
                blocks: vec![Block::ToDo(ToDo {
                    head: SString {
                        node: "Hello, FromDo!".to_string(),
                        span: Span { lo: 2, hi: 16 },
                    },
                    body: None,
                    due: Some(property::Due {
                        rel: None,
                        ts: Some(ts("2026-04-08T08:00:00+00:00[UTC]")),
                    }),
                    late_due: Some(property::Due {
                        rel: None,
                        ts: Some(ts("2026-04-09T08:00:00+00:00[UTC]")),
                    }),
                })],
            },
        );
    }

    #[test]
    fn sanity_3() {
        //|
        //|
        //| :now 2026-04-08T08:00:00+00:00[UTC]
        //|
        //| :now 2026-04-01T08:00:00+00:00[UTC]
        //|
        //| -	Hello, FromDo!
        //| 	due
        //| 		2026-04-08T08:00:00+00:00[UTC]
        //|
        //| 	What's the buzz?
        //|
        //| -	Hello, FromDo!
        //| 	due
        //| 		2026-04-01T08:00:00+00:00[UTC]
        //|
        //| 	What's the buzz?
        //|
        assert_program(
            vec![
                t!(Token::Line, "\n"),
                t!(Token::Line, "\n"),
                t!(Token::DirectiveHead, ":"),
                t!(Token::DirectiveArg, "now"),
                t!(Token::Space, " "),
                t!(Token::DirectiveArg, "2026-04-08T08:00:00+00:00[UTC]"),
                t!(Token::Line, "\n"),
                t!(Token::Line, "\n"),
                t!(Token::DirectiveHead, ":"),
                t!(Token::DirectiveArg, "now"),
                t!(Token::Space, " "),
                t!(Token::DirectiveArg, "2026-04-01T08:00:00+00:00[UTC]"),
                t!(Token::Line, "\n"),
                t!(Token::Line, "\n"),
                t!(Token::ToDoHead, "-"),
                t!(Token::ToDoIndent, "\t"),
                t!(Token::ToDoContent, "Hello, FromDo!"),
                t!(Token::Line, "\n"),
                t!(Token::ToDoIndent, "\t"),
                t!(Token::ToDoContent, "due"),
                t!(Token::Line, "\n"),
                t!(Token::ToDoIndent, "\t"),
                t!(Token::ToDoIndent, "\t"),
                t!(Token::ToDoContent, "2026-04-08T08:00:00+00:00[UTC]"),
                t!(Token::Line, "\n"),
                t!(Token::ToDoIndent, "\t"),
                t!(Token::Line, "\n"),
                t!(Token::ToDoIndent, "\t"),
                t!(Token::ToDoContent, "What's the buzz?"),
                t!(Token::Line, "\n"),
                t!(Token::Line, "\n"),
                t!(Token::ToDoHead, "-"),
                t!(Token::ToDoIndent, "\t"),
                t!(Token::ToDoContent, "Hello, FromDo!"),
                t!(Token::Line, "\n"),
                t!(Token::ToDoIndent, "\t"),
                t!(Token::ToDoContent, "due"),
                t!(Token::Line, "\n"),
                t!(Token::ToDoIndent, "\t"),
                t!(Token::ToDoIndent, "\t"),
                t!(Token::ToDoContent, "2026-04-01T08:00:00+00:00[UTC]"),
                t!(Token::Line, "\n"),
                t!(Token::ToDoIndent, "\t"),
                t!(Token::Line, "\n"),
                t!(Token::ToDoIndent, "\t"),
                t!(Token::ToDoContent, "What's the buzz?"),
                t!(Token::Line, "\n"),
                t!(Token::Line, "\n"),
                t!(Token::EOF, ""),
            ],
            Program {
                blocks: vec![
                    Block::Directive(Directive::Now(directive::Now {
                        now: ts("2026-04-08T08:00:00+00:00[UTC]"),
                    })),
                    Block::Directive(Directive::Now(directive::Now {
                        now: ts("2026-04-01T08:00:00+00:00[UTC]"),
                    })),
                    Block::ToDo(ToDo {
                        head: SString {
                            node: "Hello, FromDo!".to_string(),
                            span: Span { lo: 78, hi: 92 },
                        },
                        body: Some(SString {
                            node: "What's the buzz?\n".to_string(),
                            span: Span { lo: 134, hi: 151 },
                        }),
                        due: Some(property::Due {
                            rel: None,
                            ts: Some(ts("2026-04-08T08:00:00+00:00[UTC]")),
                        }),
                        late_due: None,
                    }),
                    Block::ToDo(ToDo {
                        head: SString {
                            node: "Hello, FromDo!".to_string(),
                            span: Span { lo: 154, hi: 168 },
                        },
                        body: Some(SString {
                            node: "What's the buzz?\n".to_string(),
                            span: Span { lo: 210, hi: 227 },
                        }),
                        due: Some(property::Due {
                            rel: None,
                            ts: Some(ts("2026-04-01T08:00:00+00:00[UTC]")),
                        }),
                        late_due: None,
                    }),
                ],
            },
        );
    }

    #[test]
    fn block_error_lexer() {
        // LexerError("What's the buzz?")
        assert_program(
            vec![t!(Token::Error, "What's the buzz?"), t!(Token::EOF, "")],
            Program {
                blocks: vec![Block::Error(Error::LexerError(SString::new(
                    "What's the buzz?",
                    0,
                    16,
                )))],
            },
        );
    }

    #[test]
    fn block_error_space() {
        // UnexpectedToken(Space)
        assert_program(
            vec![t!(Token::Space, " "), t!(Token::EOF, "")],
            Program {
                blocks: vec![Block::Error(Error::UnexpectedToken {
                    unexpected: Token::Space(SString::new(" ", 0, 1)),
                    expected: Parser::BLOCK_EXPECTED,
                })],
            },
        );
    }

    #[test]
    fn block_error_directive_arg() {
        // UnexpectedToken(DirectiveArg("FromDo"))
        assert_program(
            vec![t!(Token::DirectiveArg, "FromDo"), t!(Token::EOF, "")],
            Program {
                blocks: vec![Block::Error(Error::UnexpectedToken {
                    unexpected: Token::DirectiveArg(SString::new("FromDo", 0, 6)),
                    expected: Parser::BLOCK_EXPECTED,
                })],
            },
        );
    }

    #[test]
    fn block_error_todo_indent() {
        // UnexpectedToken(ToDoIndent)
        assert_program(
            vec![t!(Token::ToDoIndent, "\t"), t!(Token::EOF, "")],
            Program {
                blocks: vec![Block::Error(Error::UnexpectedToken {
                    unexpected: Token::ToDoIndent(SString::new("\t", 0, 1)),
                    expected: Parser::BLOCK_EXPECTED,
                })],
            },
        );
    }

    #[test]
    fn block_error_todo_content() {
        // UnexpectedToken(ToDoContent("FromDo"))
        assert_program(
            vec![t!(Token::ToDoContent, "FromDo"), t!(Token::EOF, "")],
            Program {
                blocks: vec![Block::Error(Error::UnexpectedToken {
                    unexpected: Token::ToDoContent(SString::new("FromDo", 0, 6)),
                    expected: Parser::BLOCK_EXPECTED,
                })],
            },
        );
    }

    #[test]
    fn directive_now() {
        //| :now 2026-04-08T08:00:00+00:00[UTC]
        assert_program(
            vec![
                t!(Token::DirectiveHead, ":"),
                t!(Token::DirectiveArg, "now"),
                t!(Token::Space, " "),
                t!(Token::DirectiveArg, "2026-04-08T08:00:00+00:00[UTC]"),
                t!(Token::EOF, ""),
            ],
            Program {
                blocks: vec![Block::Directive(Directive::Now(directive::Now {
                    now: ts("2026-04-08T08:00:00+00:00[UTC]"),
                }))],
            },
        );
    }

    #[test]
    fn directive_tz() {
        //| :tz America/New_York
        assert_program(
            vec![
                t!(Token::DirectiveHead, ":"),
                t!(Token::DirectiveArg, "tz"),
                t!(Token::Space, " "),
                t!(Token::DirectiveArg, "America/New_York"),
                t!(Token::EOF, ""),
            ],
            Program {
                blocks: vec![Block::Directive(Directive::Tz(directive::Tz {
                    tz: tz("America/New_York"),
                }))],
            },
        );
    }

    #[test]
    fn directive_tz_error_invalid() {
        //| :tz FromDo/Nowhere
        assert_program(
            vec![
                t!(Token::DirectiveHead, ":"),
                t!(Token::DirectiveArg, "tz"),
                t!(Token::Space, " "),
                t!(Token::DirectiveArg, "FromDo/Nowhere"),
                t!(Token::EOF, ""),
            ],
            Program {
                blocks: vec![Block::Error(Error::TimeZoneParseError {
                    time_zone: SString {
                        node: "FromDo/Nowhere".to_string(),
                        span: Span { lo: 4, hi: 18 },
                    },
                    message: TimeZone::get("FromDo/Nowhere").unwrap_err().to_string(),
                })],
            },
        );
    }

    #[test]
    fn directive_error_unknown() {
        //| :unknown
        assert_program(
            vec![
                t!(Token::DirectiveHead, ":"),
                t!(Token::DirectiveArg, "unknown"),
                t!(Token::EOF, ""),
            ],
            Program {
                blocks: vec![Block::Error(Error::UnknownDirective(SString {
                    node: "unknown".to_string(),
                    span: Span { lo: 1, hi: 8 },
                }))],
            },
        );
    }

    #[test]
    fn directive_error_no_name_eof() {
        //| :
        assert_program(
            vec![t!(Token::DirectiveHead, ":"), t!(Token::EOF, "")],
            Program {
                blocks: vec![Block::Error(Error::UnexpectedToken {
                    unexpected: Token::EOF(SString::new("", 1, 1)),
                    expected: Parser::DIRECTIVE_ARG_EXPECTED,
                })],
            },
        );
    }

    #[test]
    fn directive_error_no_name() {
        //| :
        assert_program(
            vec![
                t!(Token::DirectiveHead, ":"),
                t!(Token::Space, " "),
                t!(Token::EOF, ""),
            ],
            Program {
                blocks: vec![Block::Error(Error::UnexpectedToken {
                    unexpected: Token::Space(SString::new(" ", 1, 2)),
                    expected: Parser::DIRECTIVE_ARG_EXPECTED,
                })],
            },
        );
    }

    #[test]
    fn directive_error_no_space_1() {
        //| :now
        assert_program(
            vec![
                t!(Token::DirectiveHead, ":"),
                t!(Token::DirectiveArg, "now"),
                t!(Token::EOF, ""),
            ],
            Program {
                blocks: vec![Block::Error(Error::UnexpectedToken {
                    unexpected: Token::EOF(SString::new("", 4, 4)),
                    expected: Parser::SPACE_EXPECTED,
                })],
            },
        );
    }

    #[test]
    fn directive_error_no_space_2() {
        //| :now2026-04-08T08:00:00Z
        assert_program(
            vec![
                t!(Token::DirectiveHead, ":"),
                t!(Token::DirectiveArg, "now"),
                t!(Token::DirectiveArg, "2026-04-08T08:00:00Z"),
                t!(Token::EOF, ""),
            ],
            Program {
                blocks: vec![Block::Error(Error::UnexpectedToken {
                    unexpected: Token::DirectiveArg(SString::new("2026-04-08T08:00:00Z", 4, 24)),
                    expected: Parser::SPACE_EXPECTED,
                })],
            },
        );
    }

    #[test]
    fn directive_error_no_value() {
        //| :now
        assert_program(
            vec![
                t!(Token::DirectiveHead, ":"),
                t!(Token::DirectiveArg, "now"),
                t!(Token::Space, " "),
                t!(Token::EOF, ""),
            ],
            Program {
                blocks: vec![Block::Error(Error::UnexpectedToken {
                    unexpected: Token::EOF(SString::new("", 5, 5)),
                    expected: Parser::DIRECTIVE_ARG_EXPECTED,
                })],
            },
        );
    }

    #[test]
    fn todo_simple() {
        //| -	FromDo
        assert_program(
            vec![
                t!(Token::ToDoHead, "-"),
                t!(Token::ToDoIndent, "\t"),
                t!(Token::ToDoContent, "FromDo"),
                t!(Token::Line, "\n"),
                t!(Token::EOF, ""),
            ],
            Program {
                blocks: vec![Block::ToDo(ToDo {
                    head: SString {
                        node: "FromDo".to_string(),
                        span: Span { lo: 2, hi: 8 },
                    },
                    body: None,
                    due: None,
                    late_due: None,
                })],
            },
        );
    }

    #[test]
    fn todo_error_no_head_indent() {
        //| -FromDo
        assert_program(
            vec![
                t!(Token::ToDoHead, "-"),
                t!(Token::ToDoContent, "FromDo"),
                t!(Token::EOF, ""),
            ],
            Program {
                blocks: vec![Block::Error(Error::UnexpectedToken {
                    unexpected: Token::ToDoContent(SString::new("FromDo", 1, 7)),
                    expected: Parser::TODO_INDENT_EXPECTED,
                })],
            },
        );
    }

    #[test]
    fn todo_error_no_head_content() {
        //| -
        assert_program(
            vec![
                t!(Token::ToDoHead, "-"),
                t!(Token::ToDoIndent, "\t"),
                t!(Token::EOF, ""),
            ],
            Program {
                blocks: vec![Block::Error(Error::UnexpectedToken {
                    unexpected: Token::EOF(SString::new("", 2, 2)),
                    expected: Parser::TODO_CONTENT_EXPECTED,
                })],
            },
        );
    }

    #[test]
    fn todo_no_head_line() {
        //| -	FromDo<EOF>
        assert_program(
            vec![
                t!(Token::ToDoHead, "-"),
                t!(Token::ToDoIndent, "\t"),
                t!(Token::ToDoContent, "FromDo"),
                t!(Token::EOF, ""),
            ],
            Program {
                blocks: vec![Block::ToDo(ToDo {
                    head: SString {
                        node: "FromDo".to_string(),
                        span: Span { lo: 2, hi: 8 },
                    },
                    body: None,
                    due: None,
                    late_due: None,
                })],
            },
        );
    }

    #[test]
    fn todo_body_1() {
        //| -	FromDo
        //|
        //| 	What's the buzz?
        assert_program(
            vec![
                t!(Token::ToDoHead, "-"),
                t!(Token::ToDoIndent, "\t"),
                t!(Token::ToDoContent, "FromDo"),
                t!(Token::Line, "\n"),
                t!(Token::ToDoIndent, "\t"),
                t!(Token::Line, "\n"),
                t!(Token::ToDoIndent, "\t"),
                t!(Token::ToDoContent, "What's the buzz?"),
                t!(Token::Line, "\n"),
                t!(Token::EOF, ""),
            ],
            Program {
                blocks: vec![Block::ToDo(ToDo {
                    head: SString {
                        node: "FromDo".to_string(),
                        span: Span { lo: 2, hi: 8 },
                    },
                    body: Some(SString {
                        node: "What's the buzz?\n".to_string(),
                        span: Span { lo: 12, hi: 29 },
                    }),
                    due: None,
                    late_due: None,
                })],
            },
        );
    }

    #[test]
    fn todo_body_3() {
        //| -	FromDo
        //|
        //| 	What's the buzz?
        //| 	Tell me what's happening!
        //| 	Think about today instead
        assert_program(
            vec![
                t!(Token::ToDoHead, "-"),
                t!(Token::ToDoIndent, "\t"),
                t!(Token::ToDoContent, "FromDo"),
                t!(Token::Line, "\n"),
                t!(Token::ToDoIndent, "\t"),
                t!(Token::Line, "\n"),
                t!(Token::ToDoIndent, "\t"),
                t!(Token::ToDoContent, "What's the buzz?"),
                t!(Token::Line, "\n"),
                t!(Token::ToDoIndent, "\t"),
                t!(Token::ToDoContent, "Tell me what's happening!"),
                t!(Token::Line, "\n"),
                t!(Token::ToDoIndent, "\t"),
                t!(Token::ToDoContent, "Think about today instead"),
                t!(Token::Line, "\n"),
                t!(Token::EOF, ""),
            ],
            Program {
                blocks: vec![Block::ToDo(ToDo {
                    head: SString {
                        node: "FromDo".to_string(),
                        span: Span { lo: 2, hi: 8 },
                    },
                    body: Some(SString {
                        node: indoc! {"
                            What's the buzz?
                            Tell me what's happening!
                            Think about today instead
                        "}
                        .to_string(),
                        span: Span { lo: 12, hi: 83 },
                    }),
                    due: None,
                    late_due: None,
                })],
            },
        );
    }

    #[test]
    fn todo_body_4() {
        //| -	FromDo
        //|
        //| 	What's the buzz?
        //| 	Tell me what's happening!
        //| 	What's the buzz?
        //| 	Tell me what's happening!
        assert_program(
            vec![
                t!(Token::ToDoHead, "-"),
                t!(Token::ToDoIndent, "\t"),
                t!(Token::ToDoContent, "FromDo"),
                t!(Token::Line, "\n"),
                t!(Token::ToDoIndent, "\t"),
                t!(Token::Line, "\n"),
                t!(Token::ToDoIndent, "\t"),
                t!(Token::ToDoContent, "What's the buzz?"),
                t!(Token::Line, "\n"),
                t!(Token::ToDoIndent, "\t"),
                t!(Token::ToDoContent, "Tell me what's happening!"),
                t!(Token::Line, "\n"),
                t!(Token::ToDoIndent, "\t"),
                t!(Token::ToDoContent, "What's the buzz?"),
                t!(Token::Line, "\n"),
                t!(Token::ToDoIndent, "\t"),
                t!(Token::ToDoContent, "Tell me what's happening!"),
                t!(Token::Line, "\n"),
                t!(Token::EOF, ""),
            ],
            Program {
                blocks: vec![Block::ToDo(ToDo {
                    head: SString {
                        node: "FromDo".to_string(),
                        span: Span { lo: 2, hi: 8 },
                    },
                    body: Some(SString {
                        node: indoc! {"
                            What's the buzz?
                            Tell me what's happening!
                            What's the buzz?
                            Tell me what's happening!
                        "}
                        .to_string(),
                        span: Span { lo: 12, hi: 101 },
                    }),
                    due: None,
                    late_due: None,
                })],
            },
        );
    }

    #[test]
    fn todo_error_no_out_content() {
        //| -	FromDo
        //|
        assert_program(
            vec![
                t!(Token::ToDoHead, "-"),
                t!(Token::ToDoIndent, "\t"),
                t!(Token::ToDoContent, "FromDo"),
                t!(Token::Line, "\n"),
                t!(Token::ToDoIndent, "\t"),
                t!(Token::EOF, ""),
            ],
            Program {
                blocks: vec![Block::Error(Error::UnexpectedToken {
                    unexpected: Token::EOF(SString::new("", 10, 10)),
                    expected: Parser::TODO_CONTENT_EXPECTED,
                })],
            },
        );
    }

    #[test]
    fn todo_error_no_out_line() {
        //| -	FromDo
        //| 	What's the buzz?
        assert_program(
            vec![
                t!(Token::ToDoHead, "-"),
                t!(Token::ToDoIndent, "\t"),
                t!(Token::ToDoContent, "FromDo"),
                t!(Token::Line, "\n"),
                t!(Token::ToDoIndent, "\t"),
                t!(Token::ToDoContent, "What's the buzz?"),
                t!(Token::EOF, ""),
            ],
            Program {
                blocks: vec![Block::Error(Error::UnknownToDoProp {
                    property: SString {
                        node: "What's the buzz?".to_string(),
                        span: Span { lo: 10, hi: 26 },
                    },
                })],
            },
        );
    }

    #[test]
    fn todo_error_no_body_content() {
        //| -	FromDo
        //|
        //|
        assert_program(
            vec![
                t!(Token::ToDoHead, "-"),
                t!(Token::ToDoIndent, "\t"),
                t!(Token::ToDoContent, "FromDo"),
                t!(Token::Line, "\n"),
                t!(Token::ToDoIndent, "\t"),
                t!(Token::Line, "\n"),
                t!(Token::ToDoIndent, "\t"),
                t!(Token::EOF, ""),
            ],
            Program {
                blocks: vec![Block::Error(Error::UnexpectedToken {
                    unexpected: Token::EOF(SString::new("", 12, 12)),
                    expected: Parser::TODO_CONTENT_EXPECTED,
                })],
            },
        );
    }

    #[test]
    fn todo_no_body_line() {
        //| -	FromDo
        //|
        //| 	What's the buzz?<EOF>
        assert_program(
            vec![
                t!(Token::ToDoHead, "-"),
                t!(Token::ToDoIndent, "\t"),
                t!(Token::ToDoContent, "FromDo"),
                t!(Token::Line, "\n"),
                t!(Token::ToDoIndent, "\t"),
                t!(Token::Line, "\n"),
                t!(Token::ToDoIndent, "\t"),
                t!(Token::ToDoContent, "What's the buzz?"),
                t!(Token::EOF, ""),
            ],
            Program {
                blocks: vec![Block::ToDo(ToDo {
                    head: SString {
                        node: "FromDo".to_string(),
                        span: Span { lo: 2, hi: 8 },
                    },
                    body: Some(SString {
                        node: "What's the buzz?".to_string(),
                        span: Span { lo: 12, hi: 28 },
                    }),
                    due: None,
                    late_due: None,
                })],
            },
        );
    }

    #[test]
    fn todo_due() {
        //| -	What's the Buzz
        //| 	due
        //| 		0001-01-01T00:00:00+00:00[UTC]
        assert_program(
            vec![
                t!(Token::ToDoHead, "-"),
                t!(Token::ToDoIndent, "\t"),
                t!(Token::ToDoContent, "What's the Buzz"),
                t!(Token::Line, "\n"),
                t!(Token::ToDoIndent, "\t"),
                t!(Token::ToDoContent, "due"),
                t!(Token::Line, "\n"),
                t!(Token::ToDoIndent, "\t"),
                t!(Token::ToDoIndent, "\t"),
                t!(Token::ToDoContent, "0001-01-01T00:00:00+00:00[UTC]"),
                t!(Token::Line, "\n"),
                t!(Token::EOF, ""),
            ],
            Program {
                blocks: vec![Block::ToDo(ToDo {
                    head: SString {
                        node: "What's the Buzz".to_string(),
                        span: Span { lo: 2, hi: 17 },
                    },
                    body: None,
                    due: Some(property::Due {
                        rel: None,
                        ts: Some(ts("0001-01-01T00:00:00+00:00[UTC]")),
                    }),
                    late_due: None,
                })],
            },
        );
    }

    #[test]
    fn todo_due_error_invalid_ts() {
        //| -	What's the Buzz
        //| 	due
        //| 		0000-00-00T00:00:00Z
        assert_program(
            vec![
                t!(Token::ToDoHead, "-"),
                t!(Token::ToDoIndent, "\t"),
                t!(Token::ToDoContent, "What's the Buzz"),
                t!(Token::Line, "\n"),
                t!(Token::ToDoIndent, "\t"),
                t!(Token::ToDoContent, "due"),
                t!(Token::Line, "\n"),
                t!(Token::ToDoIndent, "\t"),
                t!(Token::ToDoIndent, "\t"),
                t!(Token::ToDoContent, "0000-00-00T00:00:00Z"),
                t!(Token::Line, "\n"),
                t!(Token::EOF, ""),
            ],
            Program {
                blocks: vec![Block::Error(Error::TimestampParseError {
                    timestamp: SString {
                        node: "0000-00-00T00:00:00Z".to_string(),
                        span: Span { lo: 25, hi: 45 },
                    },
                    message: "failed to parse month in date: failed to parse two digit integer as month: parameter 'month' is not in the required range of 1..=12".to_string(),
                })],
            },
        );
    }

    #[test]
    fn todo_due_rel() {
        //| -	What's the Buzz
        //| 	due tomorrow
        assert_program(
            vec![
                t!(Token::ToDoHead, "-"),
                t!(Token::ToDoIndent, "\t"),
                t!(Token::ToDoContent, "What's the Buzz"),
                t!(Token::Line, "\n"),
                t!(Token::ToDoIndent, "\t"),
                t!(Token::ToDoContent, "due tomorrow"),
                t!(Token::Line, "\n"),
                t!(Token::EOF, ""),
            ],
            Program {
                blocks: vec![Block::ToDo(ToDo {
                    head: SString {
                        node: "What's the Buzz".to_string(),
                        span: Span { lo: 2, hi: 17 },
                    },
                    body: None,
                    due: Some(property::Due {
                        rel: Some(from_do_cur::cur::strpcur("tomorrow").unwrap()),
                        ts: None,
                    }),
                    late_due: None,
                })],
            },
        );
    }

    #[test]
    fn todo_late_due() {
        //| -	What's the Buzz
        //| 	late due
        //| 		0001-01-01T00:00:00+00:00[UTC]
        assert_program(
            vec![
                t!(Token::ToDoHead, "-"),
                t!(Token::ToDoIndent, "\t"),
                t!(Token::ToDoContent, "What's the Buzz"),
                t!(Token::Line, "\n"),
                t!(Token::ToDoIndent, "\t"),
                t!(Token::ToDoContent, "late due"),
                t!(Token::Line, "\n"),
                t!(Token::ToDoIndent, "\t"),
                t!(Token::ToDoIndent, "\t"),
                t!(Token::ToDoContent, "0001-01-01T00:00:00+00:00[UTC]"),
                t!(Token::Line, "\n"),
                t!(Token::EOF, ""),
            ],
            Program {
                blocks: vec![Block::ToDo(ToDo {
                    head: SString {
                        node: "What's the Buzz".to_string(),
                        span: Span { lo: 2, hi: 17 },
                    },
                    body: None,
                    due: None,
                    late_due: Some(property::Due {
                        rel: None,
                        ts: Some(ts("0001-01-01T00:00:00+00:00[UTC]")),
                    }),
                })],
            },
        );
    }

    #[test]
    fn todo_unknown_prop() {
        //| -	FromDo
        //| 	What's the buzz?
        assert_program(
            vec![
                t!(Token::ToDoHead, "-"),
                t!(Token::ToDoIndent, "\t"),
                t!(Token::ToDoContent, "FromDo"),
                t!(Token::Line, "\n"),
                t!(Token::ToDoIndent, "\t"),
                t!(Token::ToDoContent, "What's the buzz?"),
                t!(Token::Line, "\n"),
                t!(Token::EOF, ""),
            ],
            Program {
                blocks: vec![Block::Error(Error::UnknownToDoProp {
                    property: SString {
                        node: "What's the buzz?".to_string(),
                        span: Span { lo: 10, hi: 26 },
                    },
                })],
            },
        );
    }

    #[test]
    fn todo_1() {
        //| -	What's the Buzz
        //| 	due
        //| 		0001-01-01T00:00:00+00:00[UTC]
        //|
        //| 	What's the buzz? Tell me what's happening
        //|
        //| 	What's the buzz? Tell me what's happening
        assert_program(
            vec![
                t!(Token::ToDoHead, "-"),
                t!(Token::ToDoIndent, "\t"),
                t!(Token::ToDoContent, "What's the Buzz"),
                t!(Token::Line, "\n"),
                t!(Token::ToDoIndent, "\t"),
                t!(Token::ToDoContent, "due"),
                t!(Token::Line, "\n"),
                t!(Token::ToDoIndent, "\t"),
                t!(Token::ToDoIndent, "\t"),
                t!(Token::ToDoContent, "0001-01-01T00:00:00+00:00[UTC]"),
                t!(Token::Line, "\n"),
                t!(Token::ToDoIndent, "\t"),
                t!(Token::Line, "\n"),
                t!(Token::ToDoIndent, "\t"),
                t!(
                    Token::ToDoContent,
                    "What's the buzz? Tell me what's happening"
                ),
                t!(Token::Line, "\n"),
                t!(Token::ToDoIndent, "\t"),
                t!(Token::Line, "\n"),
                t!(Token::ToDoIndent, "\t"),
                t!(
                    Token::ToDoContent,
                    "What's the buzz? Tell me what's happening"
                ),
                t!(Token::Line, "\n"),
                t!(Token::EOF, ""),
            ],
            Program {
                blocks: vec![Block::ToDo(ToDo {
                    head: SString {
                        node: "What's the Buzz".to_string(),
                        span: Span { lo: 2, hi: 17 },
                    },
                    body: Some(SString {
                        node: indoc! {"
                            What's the buzz? Tell me what's happening

                            What's the buzz? Tell me what's happening
                        "}
                        .to_string(),
                        span: Span { lo: 59, hi: 146 },
                    }),
                    due: Some(property::Due {
                        rel: None,
                        ts: Some(ts("0001-01-01T00:00:00+00:00[UTC]")),
                    }),
                    late_due: None,
                })],
            },
        );
    }

    #[test]
    fn todo_2() {
        //| -	What's the Buzz
        //| 	due
        //| 		0001-01-01T00:00:00+00:00[UTC]
        //|
        //| 	What's the buzz? Tell me what's happening
        //| 	What's the buzz? Tell me what's happening
        //| 	What's the buzz? Tell me what's happening
        //|
        //| 	Why should you want to know?
        //| 	Don't you mind about the future
        //| 	Don't you try to think ahead
        //| 	Save tomorrow for tomorrow
        //| 	Think about today instead
        //|
        //| 	When do we ride into Jerusalem?
        //| 	When do we ride into Jerusalem?
        //| 	When do we ride into Jerusalem?
        //|
        //| 	Let me try to cool down your face a bit
        //| 	Let me try to cool down your face a bit
        //| 	Let me try to cool down your face a bit
        assert_program(
            vec![
                t!(Token::ToDoHead, "-"),
                t!(Token::ToDoIndent, "\t"),
                t!(Token::ToDoContent, "What's the Buzz"),
                t!(Token::Line, "\n"),
                t!(Token::ToDoIndent, "\t"),
                t!(Token::ToDoContent, "due"),
                t!(Token::Line, "\n"),
                t!(Token::ToDoIndent, "\t"),
                t!(Token::ToDoIndent, "\t"),
                t!(Token::ToDoContent, "0001-01-01T00:00:00+00:00[UTC]"),
                t!(Token::Line, "\n"),
                t!(Token::ToDoIndent, "\t"),
                t!(Token::Line, "\n"),
                t!(Token::ToDoIndent, "\t"),
                t!(
                    Token::ToDoContent,
                    "What's the buzz? Tell me what's happening"
                ),
                t!(Token::Line, "\n"),
                t!(Token::ToDoIndent, "\t"),
                t!(
                    Token::ToDoContent,
                    "What's the buzz? Tell me what's happening"
                ),
                t!(Token::Line, "\n"),
                t!(Token::ToDoIndent, "\t"),
                t!(
                    Token::ToDoContent,
                    "What's the buzz? Tell me what's happening"
                ),
                t!(Token::Line, "\n"),
                t!(Token::ToDoIndent, "\t"),
                t!(Token::Line, "\n"),
                t!(Token::ToDoIndent, "\t"),
                t!(Token::ToDoContent, "Why should you want to know?"),
                t!(Token::Line, "\n"),
                t!(Token::ToDoIndent, "\t"),
                t!(Token::ToDoContent, "Don't you mind about the future"),
                t!(Token::Line, "\n"),
                t!(Token::ToDoIndent, "\t"),
                t!(Token::ToDoContent, "Don't you try to think ahead"),
                t!(Token::Line, "\n"),
                t!(Token::ToDoIndent, "\t"),
                t!(Token::ToDoContent, "Save tomorrow for tomorrow"),
                t!(Token::Line, "\n"),
                t!(Token::ToDoIndent, "\t"),
                t!(Token::ToDoContent, "Think about today instead"),
                t!(Token::Line, "\n"),
                t!(Token::ToDoIndent, "\t"),
                t!(Token::Line, "\n"),
                t!(Token::ToDoIndent, "\t"),
                t!(Token::ToDoContent, "When do we ride into Jerusalem?"),
                t!(Token::Line, "\n"),
                t!(Token::ToDoIndent, "\t"),
                t!(Token::ToDoContent, "When do we ride into Jerusalem?"),
                t!(Token::Line, "\n"),
                t!(Token::ToDoIndent, "\t"),
                t!(Token::ToDoContent, "When do we ride into Jerusalem?"),
                t!(Token::Line, "\n"),
                t!(Token::ToDoIndent, "\t"),
                t!(Token::Line, "\n"),
                t!(Token::ToDoIndent, "\t"),
                t!(
                    Token::ToDoContent,
                    "Let me try to cool down your face a bit"
                ),
                t!(Token::Line, "\n"),
                t!(Token::ToDoIndent, "\t"),
                t!(
                    Token::ToDoContent,
                    "Let me try to cool down your face a bit"
                ),
                t!(Token::Line, "\n"),
                t!(Token::ToDoIndent, "\t"),
                t!(
                    Token::ToDoContent,
                    "Let me try to cool down your face a bit"
                ),
                t!(Token::Line, "\n"),
                t!(Token::EOF, ""),
            ],
            Program {
                blocks: vec![Block::ToDo(ToDo {
                    head: SString {
                        node: "What's the Buzz".to_string(),
                        span: Span { lo: 2, hi: 17 },
                    },
                    body: Some(SString {
                        node: indoc! {"
                            What's the buzz? Tell me what's happening
                            What's the buzz? Tell me what's happening
                            What's the buzz? Tell me what's happening

                            Why should you want to know?
                            Don't you mind about the future
                            Don't you try to think ahead
                            Save tomorrow for tomorrow
                            Think about today instead

                            When do we ride into Jerusalem?
                            When do we ride into Jerusalem?
                            When do we ride into Jerusalem?

                            Let me try to cool down your face a bit
                            Let me try to cool down your face a bit
                            Let me try to cool down your face a bit
                        "}
                        .to_string(),
                        span: Span { lo: 59, hi: 563 },
                    }),
                    due: Some(property::Due {
                        rel: None,
                        ts: Some(ts("0001-01-01T00:00:00+00:00[UTC]")),
                    }),
                    late_due: None,
                })],
            },
        );
    }
}
