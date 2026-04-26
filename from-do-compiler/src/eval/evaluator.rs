use from_do_cur::cur;

use crate::parse::*;

pub struct Evaluator {
    context: Context,
}

pub struct Context {
    // TODO: parent: Option<Box<Context>>,
    now: jiff::Zoned,
    tz: jiff::tz::TimeZone,
}

impl Evaluator {
    pub fn new() -> Self {
        Self {
            context: Context {
                // TODO: parent: None,
                now: jiff::Zoned::now(),
                tz: jiff::tz::TimeZone::system(),
            },
        }
    }

    pub fn eval<'a>(&mut self, program: &'a Program) -> Result<Program, Vec<Error>> {
        let mut target = Program::new();
        let mut errors = Vec::new();
        for block in &program.blocks {
            match Self::block(block, &mut self.context) {
                Ok(block) => target.blocks.push(block),
                Err(error) => errors.push(error),
            }
        }
        if errors.is_empty() {
            Ok(target)
        } else {
            Err(errors)
        }
    }

    fn block<'a>(block: &'a Block, context: &mut Context) -> Result<Block, Error> {
        match block {
            Block::Error(error) => Err(error.clone()),
            Block::Directive(directive) => match directive {
                Directive::Now(now) => {
                    context.now = now.now.clone();
                    Ok(block.clone())
                }
                Directive::Tz(tz) => {
                    context.tz = tz.tz.clone();
                    Ok(block.clone())
                }
            },
            Block::ToDo(todo) => {
                let mut target = todo.clone();
                if let Some(due) = &todo.due {
                    match due {
                        property::Due {
                            rel: None,
                            ts: None,
                        } => {
                            panic!("invalid due property: both rel and ts are None");
                        }
                        property::Due {
                            rel: Some(rel),
                            ts: None,
                        } => {
                            let ts = rel.resolve(&context.now);
                            target.due = Some(property::Due {
                                rel: Some(rel.clone()),
                                ts: Some(ts),
                            });
                        }
                        property::Due {
                            rel: _,
                            ts: Some(ts),
                        } => {
                            target.due = Some(property::Due {
                                rel: Some(cur::Phrase::unresolve(ts, &context.now)),
                                ts: Some(ts.clone()),
                            });
                        }
                    }
                }
                if let Some(late_due) = &todo.late_due {
                    match late_due {
                        property::Due {
                            rel: None,
                            ts: None,
                        } => {
                            panic!("invalid late_due property: both rel and ts are None");
                        }
                        property::Due {
                            rel: Some(rel),
                            ts: None,
                        } => {
                            let ts = rel.resolve(&context.now);
                            target.late_due = Some(property::Due {
                                rel: Some(rel.clone()),
                                ts: Some(ts),
                            });
                        }
                        property::Due {
                            rel: _,
                            ts: Some(ts),
                        } => {
                            target.late_due = Some(property::Due {
                                rel: Some(cur::Phrase::unresolve(ts, &context.now)),
                                ts: Some(ts.clone()),
                            });
                        }
                    }
                }
                Ok(Block::ToDo(target))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lex::*;
    use pretty_assertions::assert_eq;

    fn ts(value: &str) -> jiff::Zoned {
        value.parse().unwrap()
    }

    fn tz(name: &str) -> jiff::tz::TimeZone {
        jiff::tz::TimeZone::get(name).unwrap()
    }

    fn assert_eval_ok(input: Program, expected: Program) {
        assert_eq!(Evaluator::new().eval(&input), Ok(expected));
    }

    fn assert_eval_err(input: Program, expected: Vec<Error>) {
        assert_eq!(Evaluator::new().eval(&input), Err(expected));
    }

    fn due_ts(value: &str) -> property::Due {
        property::Due {
            rel: Some(cur::Phrase::unresolve(
                &ts(value),
                &ts("2026-04-08T08:00:00+00:00[UTC]"),
            )),
            ts: Some(ts(value)),
        }
    }

    fn due_ts_ref(value: &str, reference: &str) -> property::Due {
        property::Due {
            rel: Some(cur::Phrase::unresolve(&ts(value), &ts(reference))),
            ts: Some(ts(value)),
        }
    }

    #[test]
    fn sanity_1() {
        //| :tz UTC
        //| :now 2026-04-08T08:00:00+00:00[UTC]
        //|
        //| -	Hello, FromDo!
        //| 	due
        //| 		2026-04-08T12:00:00+00:00[UTC]
        //|
        //| 	What's the buzz?
        let input = Program {
            blocks: vec![
                Block::Directive(Directive::Tz(directive::Tz { tz: tz("UTC") })),
                Block::Directive(Directive::Now(directive::Now {
                    now: ts("2026-04-08T08:00:00+00:00[UTC]"),
                })),
                Block::ToDo(ToDo {
                    head: SString {
                        span: Span { lo: 1, hi: 15 },
                        node: "Hello, FromDo!".to_string(),
                    },
                    body: Some(SString {
                        span: Span { lo: 1, hi: 18 },
                        node: "What's the buzz?\n".to_string(),
                    }),
                    due: Some(property::Due {
                        rel: None,
                        ts: Some(ts("2026-04-08T12:00:00+00:00[UTC]")),
                    }),
                    late_due: None,
                }),
            ],
        };
        assert_eval_ok(
            input,
            Program {
                blocks: vec![
                    Block::Directive(Directive::Tz(directive::Tz { tz: tz("UTC") })),
                    Block::Directive(Directive::Now(directive::Now {
                        now: ts("2026-04-08T08:00:00+00:00[UTC]"),
                    })),
                    Block::ToDo(ToDo {
                        head: SString {
                            span: Span { lo: 1, hi: 15 },
                            node: "Hello, FromDo!".to_string(),
                        },
                        body: Some(SString {
                            span: Span { lo: 1, hi: 18 },
                            node: "What's the buzz?\n".to_string(),
                        }),
                        due: Some(due_ts("2026-04-08T12:00:00+00:00[UTC]")),
                        late_due: None,
                    }),
                ],
            },
        );
    }

    #[test]
    fn directive_tz() {
        //| :tz America/New_York
        //| :now 2026-04-08T08:00:00+00:00[UTC]
        //|
        //| -	Hello, FromDo!
        //| 	due
        //| 		2026-04-08T12:00:00+00:00[UTC]
        assert_eval_ok(
            Program {
                blocks: vec![
                    Block::Directive(Directive::Tz(directive::Tz {
                        tz: tz("America/New_York"),
                    })),
                    Block::Directive(Directive::Now(directive::Now {
                        now: ts("2026-04-08T08:00:00+00:00[UTC]"),
                    })),
                    Block::ToDo(ToDo {
                        head: SString {
                            span: Span { lo: 1, hi: 15 },
                            node: "Hello, FromDo!".to_string(),
                        },
                        body: None,
                        due: Some(property::Due {
                            rel: None,
                            ts: Some(ts("2026-04-08T12:00:00+00:00[UTC]")),
                        }),
                        late_due: None,
                    }),
                ],
            },
            Program {
                blocks: vec![
                    Block::Directive(Directive::Tz(directive::Tz {
                        tz: tz("America/New_York"),
                    })),
                    Block::Directive(Directive::Now(directive::Now {
                        now: ts("2026-04-08T08:00:00+00:00[UTC]"),
                    })),
                    Block::ToDo(ToDo {
                        head: SString {
                            span: Span { lo: 1, hi: 15 },
                            node: "Hello, FromDo!".to_string(),
                        },
                        body: None,
                        due: Some(due_ts("2026-04-08T12:00:00+00:00[UTC]")),
                        late_due: None,
                    }),
                ],
            },
        );
    }

    #[test]
    fn todo_due_rel_only() {
        //| :now 2026-04-08T08:00:00+00:00[UTC]
        //|
        //| -	Hello, FromDo!
        //| 	due tomorrow
        let now = "2026-04-08T08:00:00+00:00[UTC]";
        let rel = cur::strpcur("tomorrow").unwrap();
        let input = Program {
            blocks: vec![
                Block::Directive(Directive::Now(directive::Now { now: ts(now) })),
                Block::ToDo(ToDo {
                    head: SString {
                        span: Span { lo: 1, hi: 15 },
                        node: "Hello, FromDo!".to_string(),
                    },
                    body: None,
                    due: Some(property::Due {
                        rel: Some(rel.clone()),
                        ts: None,
                    }),
                    late_due: None,
                }),
            ],
        };
        let resolved_ts = rel.resolve(&ts(now));
        assert_eval_ok(
            input,
            Program {
                blocks: vec![
                    Block::Directive(Directive::Now(directive::Now { now: ts(now) })),
                    Block::ToDo(ToDo {
                        head: SString {
                            span: Span { lo: 1, hi: 15 },
                            node: "Hello, FromDo!".to_string(),
                        },
                        body: None,
                        due: Some(property::Due {
                            rel: Some(rel),
                            ts: Some(resolved_ts),
                        }),
                        late_due: None,
                    }),
                ],
            },
        );
    }

    #[test]
    fn todo_late_due() {
        //| :now 2026-04-08T08:00:00+00:00[UTC]
        //|
        //| -	Hello, FromDo!
        //| 	late due
        //| 		2026-04-09T08:00:00+00:00[UTC]
        let now = "2026-04-08T08:00:00+00:00[UTC]";
        let input = Program {
            blocks: vec![
                Block::Directive(Directive::Now(directive::Now { now: ts(now) })),
                Block::ToDo(ToDo {
                    head: SString {
                        span: Span { lo: 1, hi: 15 },
                        node: "Hello, FromDo!".to_string(),
                    },
                    body: None,
                    due: None,
                    late_due: Some(property::Due {
                        rel: None,
                        ts: Some(ts("2026-04-09T08:00:00+00:00[UTC]")),
                    }),
                }),
            ],
        };
        assert_eval_ok(
            input,
            Program {
                blocks: vec![
                    Block::Directive(Directive::Now(directive::Now { now: ts(now) })),
                    Block::ToDo(ToDo {
                        head: SString {
                            span: Span { lo: 1, hi: 15 },
                            node: "Hello, FromDo!".to_string(),
                        },
                        body: None,
                        due: None,
                        late_due: Some(due_ts_ref("2026-04-09T08:00:00+00:00[UTC]", now)),
                    }),
                ],
            },
        );
    }

    #[test]
    fn todo_no_due() {
        //| -	FromDo
        let input = Program {
            blocks: vec![Block::ToDo(ToDo {
                head: SString {
                    span: Span { lo: 1, hi: 7 },
                    node: "FromDo".to_string(),
                },
                body: None,
                due: None,
                late_due: None,
            })],
        };
        assert_eval_ok(input.clone(), input);
    }

    #[test]
    fn error_1() {
        // Block::Error(LexerError("What's the buzz?"))
        assert_eval_err(
            Program {
                blocks: vec![Block::Error(Error::LexerError(SString::new(
                    "What's the buzz?",
                    0,
                    16,
                )))],
            },
            vec![Error::LexerError(SString::new("What's the buzz?", 0, 16))],
        );
    }
}
