use crate::lexer::*;

use std::iter::Peekable;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Block {
    Error(Error),

    Directive(Directive),
    ToDo(ToDo),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    LexerError(SString),
    UnexpectedToken {
        unexpected: Token,
        expected: &'static str,
    },
    UnexpectedEOF {
        at: Span,
        expected: &'static str,
    },
    UnknownDirective(SString),
}

use Error::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Program {
    pub blocks: Vec<Block>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Directive {
    Now(directive::Now),
}

pub mod directive {
    use super::*;

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct Now {
        pub now: SString,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToDo {
    pub head: SString,
    pub body: Option<SString>,
}

pub struct Parser<Iter>
where
    Iter: Iterator<Item = Token>,
{
    source: Peekable<Iter>,
    progress: Span,
}

type Result<T> = std::result::Result<T, Error>;

impl<Iter> Parser<Iter>
where
    Iter: Iterator<Item = Token>,
{
    pub fn new(input: Iter) -> Self {
        Self {
            source: input.peekable(),
            progress: Span { lo: 0, hi: 0 },
        }
    }

    fn next(&mut self, expected: &'static str) -> Result<Token> {
        match self.source.next() {
            Some(token) => {
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

        while let Some(node) = self.source.peek() {
            if let Token::Line(_) = node {
                self.source.next().unwrap();
                continue;
            }
            if let Token::EOF(_) = node {
                self.source.next().unwrap();
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
                let now = self.directive_arg()?;
                Ok(Directive::Now(directive::Now { now }))
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
        let _ = self.line()?;

        let mut contents = Vec::new();
        'a: while let Some(Token::ToDoIndent(_)) = self.source.peek() {
            let _ = self.todo_indent().unwrap();
            'b: loop {
                if let Some(Token::Line(_)) = self.source.peek() {
                    let line = self.line().unwrap();
                    contents.push(line);
                    continue 'a;
                } else {
                    break 'b;
                }
            }
            let content = self.todo_content()?;
            let _ = self.line()?;
            contents.push(content);
        }
        if contents.is_empty() {
            Ok(ToDo { head, body: None })
        } else {
            let body = contents.into_iter().reduce(|c1, c2| c1 + c2).unwrap();
            Ok(ToDo {
                head,
                body: Some(body),
            })
        }
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

#[cfg(test)]
mod tests {
    use super::Parser as _Parser;
    use super::*;
    use pretty_assertions::assert_eq;

    macro_rules! t {
        ($bt_name:path, $value:expr) => {
            $bt_name(SString::new($value, 0, 0))
        };
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

    type Parser = _Parser<std::vec::IntoIter<Token>>;

    #[test]
    fn sanity_1() {
        assert_program(
            vec![
                t!(Token::DirectiveHead, ":"),
                t!(Token::DirectiveArg, "now"),
                t!(Token::Space, " "),
                t!(Token::DirectiveArg, "2026-04-08T08:00:00Z"),
                t!(Token::Line, "\n\n"),
                t!(Token::ToDoHead, "-"),
                t!(Token::ToDoIndent, "\t"),
                t!(
                    Token::ToDoContent,
                    "Hello, FromDo! due 2026-04-08T08:00:00Z"
                ),
                t!(Token::Line, "\n"),
                t!(Token::EOF, ""),
            ],
            Program {
                blocks: vec![
                    Block::Directive(Directive::Now(directive::Now {
                        now: SString {
                            node: "2026-04-08T08:00:00Z".to_string(),
                            span: Span { lo: 5, hi: 25 },
                        },
                    })),
                    Block::ToDo(ToDo {
                        head: SString {
                            node: "Hello, FromDo! due 2026-04-08T08:00:00Z".to_string(),
                            span: Span { lo: 29, hi: 68 },
                        },
                        body: None,
                    }),
                ],
            },
        );
    }

    #[test]
    fn sanity_2() {
        assert_program(
            vec![
                t!(Token::Line, "\n\n"),
                t!(Token::DirectiveHead, ":"),
                t!(Token::DirectiveArg, "now"),
                t!(Token::Space, " "),
                t!(Token::DirectiveArg, "2026-04-08T08:00:00Z"),
                t!(Token::Line, "\n\n"),
                t!(Token::DirectiveHead, ":"),
                t!(Token::DirectiveArg, "now"),
                t!(Token::Space, " "),
                t!(Token::DirectiveArg, "2026-04-01T08:00:00Z"),
                t!(Token::Line, "\n\n"),
                t!(Token::ToDoHead, "-"),
                t!(Token::ToDoIndent, "\t"),
                t!(
                    Token::ToDoContent,
                    "Hello, FromDo! due 2026-04-08T08:00:00Z"
                ),
                t!(Token::Line, "\n"),
                t!(Token::ToDoIndent, "\t"),
                t!(Token::ToDoContent, "What's the buzz?"),
                t!(Token::Line, "\n\n"),
                t!(Token::ToDoHead, "-"),
                t!(Token::ToDoIndent, "\t"),
                t!(
                    Token::ToDoContent,
                    "Hello, FromDo! due 2026-04-01T08:00:00Z"
                ),
                t!(Token::Line, "\n"),
                t!(Token::ToDoIndent, "\t"),
                t!(Token::ToDoContent, "What's the buzz?"),
                t!(Token::Line, "\n\n"),
                t!(Token::EOF, ""),
            ],
            Program {
                blocks: vec![
                    Block::Directive(Directive::Now(directive::Now {
                        now: SString {
                            node: "2026-04-08T08:00:00Z".to_string(),
                            span: Span { lo: 7, hi: 27 },
                        },
                    })),
                    Block::Directive(Directive::Now(directive::Now {
                        now: SString {
                            node: "2026-04-01T08:00:00Z".to_string(),
                            span: Span { lo: 34, hi: 54 },
                        },
                    })),
                    Block::ToDo(ToDo {
                        head: SString {
                            node: "Hello, FromDo! due 2026-04-08T08:00:00Z".to_string(),
                            span: Span { lo: 58, hi: 97 },
                        },
                        body: Some(SString {
                            node: "What's the buzz?".to_string(),
                            span: Span { lo: 99, hi: 115 },
                        }),
                    }),
                    Block::ToDo(ToDo {
                        head: SString {
                            node: "Hello, FromDo! due 2026-04-01T08:00:00Z".to_string(),
                            span: Span { lo: 119, hi: 158 },
                        },
                        body: Some(SString {
                            node: "What's the buzz?".to_string(),
                            span: Span { lo: 160, hi: 176 },
                        }),
                    }),
                ],
            },
        );
    }

    #[test]
    fn block_error_lexer() {
        assert_program(
            vec![t!(Token::Error, "buzz"), t!(Token::EOF, "")],
            Program {
                blocks: vec![Block::Error(Error::LexerError(SString::new("buzz", 0, 4)))],
            },
        );
    }

    #[test]
    fn block_error_space() {
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
        assert_program(
            vec![t!(Token::DirectiveArg, "dang"), t!(Token::EOF, "")],
            Program {
                blocks: vec![Block::Error(Error::UnexpectedToken {
                    unexpected: Token::DirectiveArg(SString::new("dang", 0, 4)),
                    expected: Parser::BLOCK_EXPECTED,
                })],
            },
        );
    }

    #[test]
    fn block_error_todo_indent() {
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
        assert_program(
            vec![t!(Token::ToDoContent, "Head"), t!(Token::EOF, "")],
            Program {
                blocks: vec![Block::Error(Error::UnexpectedToken {
                    unexpected: Token::ToDoContent(SString::new("Head", 0, 4)),
                    expected: Parser::BLOCK_EXPECTED,
                })],
            },
        );
    }

    #[test]
    fn directive_now() {
        assert_program(
            vec![
                t!(Token::DirectiveHead, ":"),
                t!(Token::DirectiveArg, "now"),
                t!(Token::Space, " "),
                t!(Token::DirectiveArg, "2026-04-08T08:00:00Z"),
                t!(Token::EOF, ""),
            ],
            Program {
                blocks: vec![Block::Directive(Directive::Now(directive::Now {
                    now: SString {
                        node: "2026-04-08T08:00:00Z".to_string(),
                        span: Span { lo: 5, hi: 25 },
                    },
                }))],
            },
        );
    }

    #[test]
    fn directive_error_unknown() {
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
    fn directive_error_no_name() {
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
                })],
            },
        );
    }

    #[test]
    fn todo_error_no_head_indent() {
        assert_program(
            vec![
                t!(Token::ToDoHead, "-"),
                t!(Token::ToDoContent, "Head"),
                t!(Token::EOF, ""),
            ],
            Program {
                blocks: vec![Block::Error(Error::UnexpectedToken {
                    unexpected: Token::ToDoContent(SString::new("Head", 1, 5)),
                    expected: Parser::TODO_INDENT_EXPECTED,
                })],
            },
        );
    }

    #[test]
    fn todo_error_no_head_content() {
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
    fn todo_error_no_head_line() {
        assert_program(
            vec![
                t!(Token::ToDoHead, "-"),
                t!(Token::ToDoIndent, "\t"),
                t!(Token::ToDoContent, "Head"),
                t!(Token::EOF, ""),
            ],
            Program {
                blocks: vec![Block::Error(Error::UnexpectedToken {
                    unexpected: Token::EOF(SString::new("", 6, 6)),
                    expected: Parser::LINE_EXPECTED,
                })],
            },
        );
    }

    #[test]
    fn todo_body_1() {
        assert_program(
            vec![
                t!(Token::ToDoHead, "-"),
                t!(Token::ToDoIndent, "\t"),
                t!(Token::ToDoContent, "Head"),
                t!(Token::Line, "\n"),
                t!(Token::ToDoIndent, "\t"),
                t!(Token::ToDoContent, "Body"),
                t!(Token::Line, "\n"),
                t!(Token::EOF, ""),
            ],
            Program {
                blocks: vec![Block::ToDo(ToDo {
                    head: SString {
                        node: "Head".to_string(),
                        span: Span { lo: 2, hi: 6 },
                    },
                    body: Some(SString {
                        node: "Body".to_string(),
                        span: Span { lo: 8, hi: 12 },
                    }),
                })],
            },
        );
    }

    #[test]
    fn todo_body_3() {
        assert_program(
            vec![
                t!(Token::ToDoHead, "-"),
                t!(Token::ToDoIndent, "\t"),
                t!(Token::ToDoContent, "Head"),
                t!(Token::Line, "\n"),
                t!(Token::ToDoIndent, "\t"),
                t!(Token::ToDoContent, "Veni, "),
                t!(Token::Line, "\n"),
                t!(Token::ToDoIndent, "\t"),
                t!(Token::ToDoContent, "Vidi, "),
                t!(Token::Line, "\n"),
                t!(Token::ToDoIndent, "\t"),
                t!(Token::Line, "\n"),
                t!(Token::ToDoIndent, "\t"),
                t!(Token::ToDoContent, "Vici. "),
                t!(Token::Line, "\n"),
                t!(Token::EOF, ""),
            ],
            Program {
                blocks: vec![Block::ToDo(ToDo {
                    head: SString {
                        node: "Head".to_string(),
                        span: Span { lo: 2, hi: 6 },
                    },
                    body: Some(SString {
                        node: "Veni, Vidi, \nVici. ".to_string(),
                        span: Span { lo: 8, hi: 32 },
                    }),
                })],
            },
        );
    }

    #[test]
    fn todo_error_no_body_content() {
        assert_program(
            vec![
                t!(Token::ToDoHead, "-"),
                t!(Token::ToDoIndent, "\t"),
                t!(Token::ToDoContent, "Head"),
                t!(Token::Line, "\n"),
                t!(Token::ToDoIndent, "\t"),
                t!(Token::EOF, ""),
            ],
            Program {
                blocks: vec![Block::Error(Error::UnexpectedToken {
                    unexpected: Token::EOF(SString::new("", 8, 8)),
                    expected: Parser::TODO_CONTENT_EXPECTED,
                })],
            },
        );
    }

    #[test]
    fn todo_error_no_body_line() {
        assert_program(
            vec![
                t!(Token::ToDoHead, "-"),
                t!(Token::ToDoIndent, "\t"),
                t!(Token::ToDoContent, "Head"),
                t!(Token::Line, "\n"),
                t!(Token::ToDoIndent, "\t"),
                t!(Token::ToDoContent, "Body"),
                t!(Token::EOF, ""),
            ],
            Program {
                blocks: vec![Block::Error(Error::UnexpectedToken {
                    unexpected: Token::EOF(SString::new("", 12, 12)),
                    expected: Parser::LINE_EXPECTED,
                })],
            },
        );
    }
}
