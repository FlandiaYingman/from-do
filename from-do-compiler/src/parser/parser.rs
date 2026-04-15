use std::iter::Peekable;

use crate::lexer::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Block {
    Error { error: Error, raw: SString },

    Directive(Directive),
    ToDo(ToDo),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    LexerError,
    UnexpectedToken(Token),
    UnexpectedEOF,
    UnknownDirective(SString),
}

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
    Iter: Iterator<Item = SToken>,
{
    source: Peekable<Iter>,
}

impl<Iter> Parser<Iter>
where
    Iter: Iterator<Item = SToken>,
{
    pub fn new(input: Iter) -> Self {
        Self {
            source: input.peekable(),
        }
    }

    pub fn program(&mut self) -> Program {
        let mut blocks = Vec::new();

        while let Some(Spannable { node, .. }) = self.source.peek() {
            if let Token::EOF = node {
                self.source.next();
                break;
            }
            blocks.push(self.block());
        }

        Program { blocks }
    }

    fn block(&mut self) -> Block {
        let Spannable { node, span } = self.source.next().unwrap();

        match node {
            Token::Error(s) => Block::Error {
                error: Error::LexerError,
                raw: SString { node: s, span },
            },
            Token::EOF => Block::Error {
                error: Error::UnexpectedToken(Token::EOF),
                raw: SString {
                    node: "".into(),
                    span,
                },
            },

            Token::Line(_) => self.block(),
            Token::Space(s) => Block::Error {
                error: Error::UnexpectedToken(Token::Space(s.clone())),
                raw: SString { node: s, span },
            },

            Token::DirectiveHead(head) => self.directive().map_or_else(
                |err| Block::Error {
                    error: err,
                    raw: SString { node: head, span },
                },
                |directive| Block::Directive(directive),
            ),
            Token::DirectiveArg(s) => Block::Error {
                error: Error::UnexpectedToken(Token::DirectiveArg(s.clone())),
                raw: SString { node: s, span },
            },

            Token::ToDoHead(_) => self.todo().map_or_else(
                |err| Block::Error {
                    error: err,
                    raw: SString {
                        node: "".into(),
                        span,
                    },
                },
                |todo| Block::ToDo(todo),
            ),
            Token::ToDoIndent(s) => Block::Error {
                error: Error::UnexpectedToken(Token::ToDoIndent(s.clone())),
                raw: SString { node: s, span },
            },
            Token::ToDoContent(s) => Block::Error {
                error: Error::UnexpectedToken(Token::ToDoContent(s.clone())),
                raw: SString { node: s, span },
            },
        }
    }

    fn line(&mut self) -> Result<SString, Error> {
        let Some(Spannable { node, span }) = self.source.next() else {
            return Err(Error::UnexpectedEOF);
        };
        match node {
            Token::Line(s) => Ok(SString { node: s, span }),
            token => Err(Error::UnexpectedToken(token)),
        }
    }

    fn space(&mut self) -> Result<SToken, Error> {
        let Some(Spannable { node, span }) = self.source.next() else {
            return Err(Error::UnexpectedEOF);
        };
        match node {
            Token::Space(s) => Ok(SToken {
                node: Token::Space(s),
                span,
            }),
            token => Err(Error::UnexpectedToken(token)),
        }
    }

    fn directive(&mut self) -> Result<Directive, Error> {
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

    fn directive_arg(&mut self) -> Result<SString, Error> {
        let Some(Spannable { node, span }) = self.source.next() else {
            return Err(Error::UnexpectedEOF);
        };
        match node {
            Token::DirectiveArg(s) => Ok(SString { node: s, span }),
            token => Err(Error::UnexpectedToken(token)),
        }
    }

    fn todo(&mut self) -> Result<ToDo, Error> {
        let _ = self.todo_indent()?;
        let head = self.todo_content()?;
        let _ = self.line()?;

        let mut contents = Vec::new();
        while let Some(Spannable {
            node: Token::ToDoIndent(_),
            ..
        }) = self.source.peek()
        {
            let _ = self.todo_indent()?;
            let content = self.todo_content()?;
            let _ = self.line()?;
            contents.push(content);
        }
        if contents.len() > 0 {
            let body = contents.into_iter().reduce(|c1, c2| c1 + c2).unwrap();
            Ok(ToDo {
                head,
                body: Some(body),
            })
        } else {
            Ok(ToDo { head, body: None })
        }
    }

    fn todo_indent(&mut self) -> Result<SString, Error> {
        let Some(Spannable { node, span }) = self.source.next() else {
            return Err(Error::UnexpectedEOF);
        };
        match node {
            Token::ToDoIndent(s) => Ok(SString { node: s, span }),
            token => Err(Error::UnexpectedToken(token)),
        }
    }

    fn todo_content(&mut self) -> Result<SString, Error> {
        let Some(Spannable { node, span }) = self.source.next() else {
            return Err(Error::UnexpectedEOF);
        };
        match node {
            Token::ToDoContent(s) => Ok(SString { node: s, span }),
            token => Err(Error::UnexpectedToken(token)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn auto_span_t(bt: impl Iterator<Item = Token>) -> impl Iterator<Item = SToken> {
        let mut current = 0;
        bt.map(move |token| {
            let len = match &token {
                Token::EOF => 0,
                Token::Error(v) => v.len(),
                Token::Line(v) => v.len(),
                Token::Space(v) => v.len(),
                Token::DirectiveHead(v) => v.len(),
                Token::DirectiveArg(v) => v.len(),
                Token::ToDoHead(v) => v.len(),
                Token::ToDoIndent(v) => v.len(),
                Token::ToDoContent(v) => v.len(),
            };
            let span = Span {
                lo: current,
                hi: current + len,
            };
            current += len;
            Spannable { node: token, span }
        })
    }

    fn assert(input: Vec<Token>, should_be: Program) {
        let program = Parser::new(auto_span_t(input.into_iter())).program();
        assert_eq!(program, should_be);
    }

    #[test]
    fn sanity() {
        assert(
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
    fn directive_now() {
        assert(
            vec![
                Token::DirectiveHead(":".to_string()),
                Token::DirectiveArg("now".to_string()),
                Token::Space(" ".to_string()),
                Token::DirectiveArg("2026-04-08T08:00:00Z".to_string()),
                Token::EOF,
            ],
            Program {
                blocks: vec![Block::Directive(Directive::Now(directive::Now {
                    now: SString {
                        node: "2026-04-08T08:00:00Z".to_string(),
                        span: Span { lo: 5, hi: 25 },
                    },
                }))],
            },
        )
    }

    #[test]
    fn directive_unknown() {
        assert(
            vec![
                Token::DirectiveHead(":".to_string()),
                Token::DirectiveArg("unknown".to_string()),
                Token::EOF,
            ],
            Program {
                blocks: vec![Block::Error {
                    error: Error::UnknownDirective(SString {
                        node: "unknown".to_string(),
                        span: Span { lo: 1, hi: 8 },
                    }),
                    raw: SString {
                        node: ":".to_string(),
                        span: Span { lo: 0, hi: 1 },
                    },
                }],
            },
        );
    }

    #[test]
    fn todo_simple() {
        assert(
            vec![
                Token::ToDoHead("-".to_string()),
                Token::ToDoIndent("\t".to_string()),
                Token::ToDoContent("FromDo".to_string()),
                Token::Line("\n".to_string()),
                Token::EOF,
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
        )
    }

    #[test]
    fn todo_body_1() {
        assert(
            vec![
                Token::ToDoHead("-".to_string()),
                Token::ToDoIndent("\t".to_string()),
                Token::ToDoContent("Head".to_string()),
                Token::Line("\n".to_string()),
                Token::ToDoIndent("\t".to_string()),
                Token::ToDoContent("Body".to_string()),
                Token::Line("\n".to_string()),
                Token::EOF,
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
        )
    }

    #[test]
    fn todo_body_3() {
        assert(
            vec![
                Token::ToDoHead("-".to_string()),
                Token::ToDoIndent("\t".to_string()),
                Token::ToDoContent("Head".to_string()),
                Token::Line("\n".to_string()),
                Token::ToDoIndent("\t".to_string()),
                Token::ToDoContent("One".to_string()),
                Token::Line("\n".to_string()),
                Token::ToDoIndent("\t".to_string()),
                Token::ToDoContent("Two".to_string()),
                Token::Line("\n".to_string()),
                Token::ToDoIndent("\t".to_string()),
                Token::ToDoContent("Three".to_string()),
                Token::Line("\n".to_string()),
                Token::EOF,
            ],
            Program {
                blocks: vec![Block::ToDo(ToDo {
                    head: SString {
                        node: "Head".to_string(),
                        span: Span { lo: 2, hi: 6 },
                    },
                    body: Some(SString {
                        node: "OneTwoThree".to_string(),
                        span: Span { lo: 8, hi: 23 },
                    }),
                })],
            },
        )
    }
}
