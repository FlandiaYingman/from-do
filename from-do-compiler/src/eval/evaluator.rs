use crate::lex::*;
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

    fn format_due(now: &jiff::Zoned, due: &jiff::Zoned) -> String {
        let span = now
            .until(
                jiff::ZonedDifference::new(due)
                    .largest(jiff::Unit::Hour)
                    .smallest(jiff::Unit::Second)
                    .mode(jiff::RoundMode::Trunc),
            )
            .unwrap();
        let absolute = span.abs();

        let amount = if absolute
            .compare(jiff::Span::new().hours(24))
            .unwrap()
            .is_gt()
        {
            let span = absolute
                .round(
                    jiff::SpanRound::new()
                        .largest(jiff::Unit::Day)
                        .smallest(jiff::Unit::Hour)
                        .mode(jiff::RoundMode::Trunc)
                        .days_are_24_hours(),
                )
                .unwrap();
            format!("{}d {}h", span.get_days(), span.get_hours())
        } else if absolute
            .compare(jiff::Span::new().hours(1))
            .unwrap()
            .is_gt()
        {
            let span = absolute
                .round(
                    jiff::SpanRound::new()
                        .largest(jiff::Unit::Hour)
                        .smallest(jiff::Unit::Minute)
                        .mode(jiff::RoundMode::Trunc),
                )
                .unwrap();
            format!("{}h {}m", span.get_hours(), span.get_minutes())
        } else {
            let span = absolute
                .round(
                    jiff::SpanRound::new()
                        .largest(jiff::Unit::Minute)
                        .smallest(jiff::Unit::Second)
                        .mode(jiff::RoundMode::Trunc),
                )
                .unwrap();
            format!("{}m {}s", span.get_minutes(), span.get_seconds())
        };

        if span.is_negative() {
            format!("(due {amount} ago)")
        } else {
            format!("(due in {amount})")
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
                    target.due = Some(due.with_time_zone(context.tz.clone()));
                    target.out = Some(SString {
                        span: Span { lo: 0, hi: 0 }, // TODO: meaningless span
                        node: Self::format_due(&context.now, due),
                    });
                }
                Ok(Block::ToDo(target))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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

    fn assert_format_due(now: &str, due: &str, expected: &str) {
        assert_eq!(Evaluator::format_due(&ts(now), &ts(due)), expected);
    }

    #[test]
    fn sanity_1() {
        //| :tz UTC
        //| :now 2026-04-08T08:00:00+00:00[UTC]
        //|
        //| -	Hello, FromDo! due 2026-04-08T12:00:00+00:00[UTC]
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
                    body: None,
                    due: Some(ts("2026-04-08T12:00:00+00:00[UTC]")),
                    out: Some(SString {
                        span: Span { lo: 1, hi: 17 },
                        node: "What's the buzz?".to_string(),
                    }),
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
                        body: None,
                        due: Some(ts("2026-04-08T12:00:00+00:00[UTC]")),
                        out: Some(SString {
                            span: Span { lo: 0, hi: 0 },
                            node: "(due in 4h 0m)".to_string(),
                        }),
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
        //| -	Hello, FromDo! due 2026-04-08T08:00:00-04:00[America/New_York]
        //| 	(due in 4h 0m)
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
                        due: Some(ts("2026-04-08T12:00:00+00:00[UTC]")),
                        out: None,
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
                        due: Some(ts("2026-04-08T08:00:00-04:00[America/New_York]")),
                        out: Some(SString {
                            span: Span { lo: 0, hi: 0 },
                            node: "(due in 4h 0m)".to_string(),
                        }),
                    }),
                ],
            },
        );
    }

    #[test]
    fn todo_due_in_1() {
        //| :now 2026-04-08T08:00:00+00:00[UTC]
        //|
        //| -	Hello, FromDo! due 2026-04-15T08:00:00+00:00[UTC]
        //| 	(due in 7d 0h)
        assert_format_due(
            "2026-04-08T08:00:00+00:00[UTC]",
            "2026-04-15T08:00:00+00:00[UTC]",
            "(due in 7d 0h)",
        );
    }

    #[test]
    fn todo_due_in_2() {
        //| :now 2026-04-08T08:00:00+00:00[UTC]
        //|
        //| -	Hello, FromDo! due 2026-04-09T08:00:00+00:00[UTC]
        //| 	(due in 24h 0m)
        assert_format_due(
            "2026-04-08T08:00:00+00:00[UTC]",
            "2026-04-09T08:00:00+00:00[UTC]",
            "(due in 24h 0m)",
        );
    }

    #[test]
    fn todo_due_in_3() {
        //| :now 2026-04-08T08:00:00+00:00[UTC]
        //|
        //| -	Hello, FromDo! due 2026-04-08T09:00:00+00:00[UTC]
        //| 	(due in 60m 0s)
        assert_format_due(
            "2026-04-08T08:00:00+00:00[UTC]",
            "2026-04-08T09:00:00+00:00[UTC]",
            "(due in 60m 0s)",
        );
    }

    #[test]
    fn todo_due_ago_1() {
        //| :now 2026-04-08T12:30:00+00:00[UTC]
        //|
        //| -	Hello, FromDo! due 2026-04-08T08:00:00+00:00[UTC]
        //| 	(due 4h 30m ago)
        assert_format_due(
            "2026-04-08T12:30:00+00:00[UTC]",
            "2026-04-08T08:00:00+00:00[UTC]",
            "(due 4h 30m ago)",
        );
    }

    #[test]
    fn todo_out_1() {
        //| -	FromDo
        //| 	What's the buzz?
        let input = Program {
            blocks: vec![Block::ToDo(ToDo {
                head: SString {
                    span: Span { lo: 1, hi: 7 },
                    node: "FromDo".to_string(),
                },
                body: None,
                due: None,
                out: Some(SString {
                    span: Span { lo: 1, hi: 17 },
                    node: "What's the buzz?".to_string(),
                }),
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
