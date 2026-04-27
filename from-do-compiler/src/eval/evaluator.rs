use from_do_cur::cur;

use crate::parse::*;

pub struct Evaluator {
    context: Context,
}

pub struct Context {
    // TODO: parent: Option<Box<Context>>,
    now: jiff::Zoned,
    tz: jiff::tz::TimeZone,
    ahead: u32,
}

impl Evaluator {
    pub fn new() -> Self {
        Self {
            context: Context {
                // TODO: parent: None,
                now: jiff::Zoned::now()
                    .with()
                    .time(jiff::civil::Time::MAX)
                    .subsec_nanosecond(0)
                    .build()
                    .unwrap(),
                tz: jiff::tz::TimeZone::system(),
                ahead: 3,
            },
        }
    }

    pub fn eval<'a>(&mut self, program: &'a Program) -> Result<Program, Vec<Error>> {
        let mut target = Program::new();
        let mut errors = Vec::new();
        for block in &program.blocks {
            match Self::block(block, &mut self.context) {
                Ok(blocks) => target.blocks.extend(blocks),
                Err(error) => errors.push(error),
            }
        }
        if errors.is_empty() {
            Ok(target)
        } else {
            Err(errors)
        }
    }

    fn block<'a>(block: &'a Block, context: &mut Context) -> Result<Vec<Block>, Error> {
        match block {
            Block::Error(error) => Err(error.clone()),
            Block::Directive(directive) => match directive {
                Directive::Now(now) => {
                    context.now = now.now.clone();
                    Ok(vec![block.clone()])
                }
                Directive::Tz(tz) => {
                    context.tz = tz.tz.clone();
                    Ok(vec![block.clone()])
                }
                Directive::Ahead(ahead) => {
                    context.ahead = ahead.ahead;
                    Ok(vec![block.clone()])
                }
            },
            Block::ToDo(todo) => match &todo.schedule {
                Schedule::Once { due, late_due } => {
                    let schedule = Schedule::Once {
                        due: due.as_ref().map(|d| Self::resolve_due(d, context)),
                        late_due: late_due.as_ref().map(|d| Self::resolve_due(d, context)),
                    };
                    Ok(vec![Block::ToDo(ToDo {
                        t: todo.t.clone(),
                        head: todo.head.clone(),
                        body: todo.body.clone(),
                        schedule,
                    })])
                }
                Schedule::Recurring {
                    recurring,
                    begin,
                    until,
                } => {
                    let recurring = property::Recurring {
                        pattern: recurring.pattern.clone().normalized(),
                        ts: recurring.ts.clone(),
                    };
                    let begin = begin.as_ref().map(|d| Self::resolve_due(d, context));
                    let until = until.as_ref().map(|d| Self::resolve_due(d, context));

                    let (chidlren, ts) = if matches!(todo.t, ToDoType::ToDo) {
                        let recurrence = Self::resolve_recurring(
                            &recurring,
                            &begin.as_ref().and_then(|d| d.ts.clone()),
                            &until.as_ref().and_then(|d| d.ts.clone()),
                            context,
                        );
                        let ts = recurrence
                            .last()
                            .and_then(|x| x.ts.clone())
                            .or(recurring.ts.clone());
                        (recurrence, ts)
                    } else {
                        (vec![], recurring.ts.clone())
                    };

                    let generator = Block::ToDo(ToDo {
                        t: todo.t.clone(),
                        head: todo.head.clone(),
                        body: todo.body.clone(),
                        schedule: Schedule::Recurring {
                            recurring: property::Recurring {
                                pattern: recurring.pattern.clone(),
                                ts,
                            },
                            begin: begin.clone(),
                            until: until.clone(),
                        },
                    });

                    let children = chidlren.into_iter().map(|due| {
                        Block::ToDo(ToDo {
                            t: ToDoType::ToDo,
                            head: todo.head.clone(),
                            body: todo.body.clone(),
                            schedule: Schedule::Once {
                                due: Some(due),
                                late_due: None,
                            },
                        })
                    });

                    let mut blocks = vec![generator];
                    blocks.extend(children);
                    Ok(blocks)
                }
            },
        }
    }

    /// Resolve a `Due` so that both `rel` and `ts` are populated relative to
    /// `context.now`. If both `rel` and `ts` are already populated, the `rel`
    /// is re-resolved from the `ts` to ensure it's consistent with the
    /// `context.now`. If both `rel` and `ts` are None, it's an error and
    /// panics. If one of `rel` and `ts` is populated, the other is resolved
    /// from it.
    fn resolve_due(due: &property::Due, context: &Context) -> property::Due {
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
            } => property::Due {
                rel: Some(rel.clone()),
                ts: Some(rel.resolve(&context.now)),
            },
            property::Due {
                rel: _,
                ts: Some(ts),
            } => property::Due {
                rel: Some(cur::Phrase::unresolve(ts, &context.now)),
                ts: Some(ts.clone()),
            },
        }
    }

    // Resolve a `Recurring` so that it generates at most the next
    // `content.ahead` `Due` instances relative to `context.now`. The generated
    // `Due` instances doesn't exceed the `begin` and `until` properties. If the
    // `Recurring` has a `ts`, only the difference between the instances
    // generated relative to the `ts` and the `context.now` are resolved.
    fn resolve_recurring(
        recurring: &property::Recurring,
        begin: &Option<jiff::Zoned>,
        until: &Option<jiff::Zoned>,
        context: &Context,
    ) -> Vec<property::Due> {
        let mut vec = Vec::new();
        let mut now = match &recurring.ts {
            Some(ts) if *ts > context.now => ts.clone(),
            _ => context.now.clone(),
        };
        for _ in 0..context.ahead {
            let n = recurring.pattern.next(&now);
            if let Some(n) = n {
                if let Some(ts) = until
                    && n > *ts
                {
                    break;
                }
                if let Some(ts) = begin
                    && n < *ts
                {
                    now = n.clone();
                    continue;
                }
                vec.push(Self::resolve_due(
                    &property::Due {
                        rel: None,
                        ts: Some(n.clone()),
                    },
                    context,
                ));
                now = n;
            } else {
                break;
            }
        }
        vec
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

    fn ss(s: &str) -> SString {
        SString {
            span: Span { lo: 0, hi: s.len() },
            node: s.to_string(),
        }
    }

    fn assert_eval_ok(input: Program, expected: Program) {
        assert_eq!(Evaluator::new().eval(&input), Ok(expected));
    }

    fn assert_eval_err(input: Program, expected: Vec<Error>) {
        assert_eq!(Evaluator::new().eval(&input), Err(expected));
    }

    fn due_ts_ref(value: &str, ref_now: &str) -> property::Due {
        property::Due {
            rel: Some(cur::Phrase::unresolve(&ts(value), &ts(ref_now))),
            ts: Some(ts(value)),
        }
    }

    /// `due_ts_ref` with the canonical sanity-test `now`.
    fn due_ts(value: &str) -> property::Due {
        due_ts_ref(value, "2026-04-08T08:00:00+00:00[UTC]")
    }

    /// A generated recurring-child to-do block.
    fn child_block(head_node: &str, ts_str: &str, now: &str) -> Block {
        Block::ToDo(ToDo {
            t: ToDoType::ToDo,
            head: ss(head_node),
            body: None,
            schedule: Schedule::Once {
                due: Some(due_ts_ref(ts_str, now)),
                late_due: None,
            },
        })
    }

    /// A `recurring` schedule with no `begin`/`until`/last-`ts`.
    fn recurring_schedule(pattern: &str) -> Schedule {
        Schedule::Recurring {
            recurring: property::Recurring {
                pattern: from_do_cur::recur::strprecur(pattern).unwrap(),
                ts: None,
            },
            begin: None,
            until: None,
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
                    t: ToDoType::ToDo,
                    head: SString {
                        span: Span { lo: 1, hi: 15 },
                        node: "Hello, FromDo!".to_string(),
                    },
                    body: Some(SString {
                        span: Span { lo: 1, hi: 18 },
                        node: "What's the buzz?\n".to_string(),
                    }),
                    schedule: Schedule::Once {
                        due: Some(property::Due {
                            rel: None,
                            ts: Some(ts("2026-04-08T12:00:00+00:00[UTC]")),
                        }),
                        late_due: None,
                    },
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
                        t: ToDoType::ToDo,
                        head: SString {
                            span: Span { lo: 1, hi: 15 },
                            node: "Hello, FromDo!".to_string(),
                        },
                        body: Some(SString {
                            span: Span { lo: 1, hi: 18 },
                            node: "What's the buzz?\n".to_string(),
                        }),
                        schedule: Schedule::Once {
                            due: Some(due_ts("2026-04-08T12:00:00+00:00[UTC]")),
                            late_due: None,
                        },
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
                        t: ToDoType::ToDo,
                        head: SString {
                            span: Span { lo: 1, hi: 15 },
                            node: "Hello, FromDo!".to_string(),
                        },
                        body: None,
                        schedule: Schedule::Once {
                            due: Some(property::Due {
                                rel: None,
                                ts: Some(ts("2026-04-08T12:00:00+00:00[UTC]")),
                            }),
                            late_due: None,
                        },
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
                        t: ToDoType::ToDo,
                        head: SString {
                            span: Span { lo: 1, hi: 15 },
                            node: "Hello, FromDo!".to_string(),
                        },
                        body: None,
                        schedule: Schedule::Once {
                            due: Some(due_ts("2026-04-08T12:00:00+00:00[UTC]")),
                            late_due: None,
                        },
                    }),
                ],
            },
        );
    }

    #[test]
    fn directive_ahead() {
        //| :ahead 5
        assert_eval_ok(
            Program {
                blocks: vec![Block::Directive(Directive::Ahead(directive::Ahead {
                    ahead: 5,
                }))],
            },
            Program {
                blocks: vec![Block::Directive(Directive::Ahead(directive::Ahead {
                    ahead: 5,
                }))],
            },
        );
    }

    #[test]
    fn todo_no_due() {
        //| -	FromDo
        let input = Program {
            blocks: vec![Block::ToDo(ToDo {
                t: ToDoType::ToDo,
                head: SString {
                    span: Span { lo: 1, hi: 7 },
                    node: "FromDo".to_string(),
                },
                body: None,
                schedule: Schedule::never(),
            })],
        };
        assert_eval_ok(input.clone(), input);
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
                    t: ToDoType::ToDo,
                    head: SString {
                        span: Span { lo: 1, hi: 15 },
                        node: "Hello, FromDo!".to_string(),
                    },
                    body: None,
                    schedule: Schedule::Once {
                        due: Some(property::Due {
                            rel: Some(rel.clone()),
                            ts: None,
                        }),
                        late_due: None,
                    },
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
                        t: ToDoType::ToDo,
                        head: SString {
                            span: Span { lo: 1, hi: 15 },
                            node: "Hello, FromDo!".to_string(),
                        },
                        body: None,
                        schedule: Schedule::Once {
                            due: Some(property::Due {
                                rel: Some(rel),
                                ts: Some(resolved_ts),
                            }),
                            late_due: None,
                        },
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
                    t: ToDoType::ToDo,
                    head: SString {
                        span: Span { lo: 1, hi: 15 },
                        node: "Hello, FromDo!".to_string(),
                    },
                    body: None,
                    schedule: Schedule::Once {
                        due: None,
                        late_due: Some(property::Due {
                            rel: None,
                            ts: Some(ts("2026-04-09T08:00:00+00:00[UTC]")),
                        }),
                    },
                }),
            ],
        };
        assert_eval_ok(
            input,
            Program {
                blocks: vec![
                    Block::Directive(Directive::Now(directive::Now { now: ts(now) })),
                    Block::ToDo(ToDo {
                        t: ToDoType::ToDo,
                        head: SString {
                            span: Span { lo: 1, hi: 15 },
                            node: "Hello, FromDo!".to_string(),
                        },
                        body: None,
                        schedule: Schedule::Once {
                            due: None,
                            late_due: Some(due_ts_ref("2026-04-09T08:00:00+00:00[UTC]", now)),
                        },
                    }),
                ],
            },
        );
    }

    #[test]
    fn todo_recurring_default() {
        //| :now 2026-04-08T08:00:00+00:00[UTC]
        //|
        //| -	FromDo
        //| 	recurring every Mon
        // Default :ahead is 3, so three children are emitted and the
        // generator's recurring.ts is bumped to the last child.
        let now = "2026-04-08T08:00:00+00:00[UTC]";
        let last = "2026-04-27T08:00:00+00:00[UTC]";
        let input = Program {
            blocks: vec![
                Block::Directive(Directive::Now(directive::Now { now: ts(now) })),
                Block::ToDo(ToDo {
                    t: ToDoType::ToDo,
                    head: ss("FromDo"),
                    body: None,
                    schedule: recurring_schedule("every Mon"),
                }),
            ],
        };
        assert_eval_ok(
            input,
            Program {
                blocks: vec![
                    Block::Directive(Directive::Now(directive::Now { now: ts(now) })),
                    Block::ToDo(ToDo {
                        t: ToDoType::ToDo,
                        head: ss("FromDo"),
                        body: None,
                        schedule: Schedule::Recurring {
                            recurring: property::Recurring {
                                pattern: from_do_cur::recur::strprecur("every Mon").unwrap(),
                                ts: Some(ts(last)),
                            },
                            begin: None,
                            until: None,
                        },
                    }),
                    child_block("FromDo", "2026-04-13T08:00:00+00:00[UTC]", now),
                    child_block("FromDo", "2026-04-20T08:00:00+00:00[UTC]", now),
                    child_block("FromDo", last, now),
                ],
            },
        );
    }

    #[test]
    fn todo_recurring_ahead_directive() {
        //| :now 2026-04-08T08:00:00+00:00[UTC]
        //| :ahead 1
        //|
        //| -	FromDo
        //| 	recurring every Mon
        let now = "2026-04-08T08:00:00+00:00[UTC]";
        let last = "2026-04-13T08:00:00+00:00[UTC]";
        let input = Program {
            blocks: vec![
                Block::Directive(Directive::Now(directive::Now { now: ts(now) })),
                Block::Directive(Directive::Ahead(directive::Ahead { ahead: 1 })),
                Block::ToDo(ToDo {
                    t: ToDoType::ToDo,
                    head: ss("FromDo"),
                    body: None,
                    schedule: recurring_schedule("every Mon"),
                }),
            ],
        };
        assert_eval_ok(
            input,
            Program {
                blocks: vec![
                    Block::Directive(Directive::Now(directive::Now { now: ts(now) })),
                    Block::Directive(Directive::Ahead(directive::Ahead { ahead: 1 })),
                    Block::ToDo(ToDo {
                        t: ToDoType::ToDo,
                        head: ss("FromDo"),
                        body: None,
                        schedule: Schedule::Recurring {
                            recurring: property::Recurring {
                                pattern: from_do_cur::recur::strprecur("every Mon").unwrap(),
                                ts: Some(ts(last)),
                            },
                            begin: None,
                            until: None,
                        },
                    }),
                    child_block("FromDo", last, now),
                ],
            },
        );
    }

    #[test]
    fn todo_recurring_ahead_zero() {
        //| :now 2026-04-08T08:00:00+00:00[UTC]
        //| :ahead 0
        //|
        //| -	FromDo
        //| 	recurring every Mon
        // No children are emitted; recurring.ts stays None.
        let now = "2026-04-08T08:00:00+00:00[UTC]";
        let input = Program {
            blocks: vec![
                Block::Directive(Directive::Now(directive::Now { now: ts(now) })),
                Block::Directive(Directive::Ahead(directive::Ahead { ahead: 0 })),
                Block::ToDo(ToDo {
                    t: ToDoType::ToDo,
                    head: ss("FromDo"),
                    body: None,
                    schedule: recurring_schedule("every Mon"),
                }),
            ],
        };
        assert_eval_ok(
            input,
            Program {
                blocks: vec![
                    Block::Directive(Directive::Now(directive::Now { now: ts(now) })),
                    Block::Directive(Directive::Ahead(directive::Ahead { ahead: 0 })),
                    Block::ToDo(ToDo {
                        t: ToDoType::ToDo,
                        head: ss("FromDo"),
                        body: None,
                        schedule: recurring_schedule("every Mon"),
                    }),
                ],
            },
        );
    }

    #[test]
    fn todo_recurring_with_from() {
        //| :now 2026-04-08T08:00:00+00:00[UTC]
        //|
        //| -	FromDo
        //| 	recurring every Mon
        //| 	begin 2026-04-20T08:00:00+00:00[UTC]
        // `begin` is itself a Monday, so it's the first occurrence (inclusive).
        // for `ahead = 3` iter 1 produces Apr 13 which is filtered by `begin`
        // (consuming one budget slot), then Apr 20 and Apr 27 are emitted.
        let now = "2026-04-08T08:00:00+00:00[UTC]";
        let begin_ts = "2026-04-20T08:00:00+00:00[UTC]";
        let last = "2026-04-27T08:00:00+00:00[UTC]";
        let input = Program {
            blocks: vec![
                Block::Directive(Directive::Now(directive::Now { now: ts(now) })),
                Block::ToDo(ToDo {
                    t: ToDoType::ToDo,
                    head: ss("FromDo"),
                    body: None,
                    schedule: Schedule::Recurring {
                        recurring: property::Recurring {
                            pattern: from_do_cur::recur::strprecur("every Mon").unwrap(),
                            ts: None,
                        },
                        begin: Some(property::Due {
                            rel: None,
                            ts: Some(ts(begin_ts)),
                        }),
                        until: None,
                    },
                }),
            ],
        };
        assert_eval_ok(
            input,
            Program {
                blocks: vec![
                    Block::Directive(Directive::Now(directive::Now { now: ts(now) })),
                    Block::ToDo(ToDo {
                        t: ToDoType::ToDo,
                        head: ss("FromDo"),
                        body: None,
                        schedule: Schedule::Recurring {
                            recurring: property::Recurring {
                                pattern: from_do_cur::recur::strprecur("every Mon").unwrap(),
                                ts: Some(ts(last)),
                            },
                            begin: Some(due_ts_ref(begin_ts, now)),
                            until: None,
                        },
                    }),
                    child_block("FromDo", "2026-04-20T08:00:00+00:00[UTC]", now),
                    child_block("FromDo", "2026-04-27T08:00:00+00:00[UTC]", now),
                ],
            },
        );
    }

    #[test]
    fn todo_recurring_with_until_truncates() {
        //| :now 2026-04-08T08:00:00+00:00[UTC]
        //|
        //| -	FromDo
        //| 	recurring every Mon
        //| 	until 2026-04-21T00:00:00+00:00[UTC]
        // Only Apr 13 and Apr 20 fall before Apr 21.
        let now = "2026-04-08T08:00:00+00:00[UTC]";
        let until_ts = "2026-04-21T00:00:00+00:00[UTC]";
        let last = "2026-04-20T08:00:00+00:00[UTC]";
        let input = Program {
            blocks: vec![
                Block::Directive(Directive::Now(directive::Now { now: ts(now) })),
                Block::ToDo(ToDo {
                    t: ToDoType::ToDo,
                    head: ss("FromDo"),
                    body: None,
                    schedule: Schedule::Recurring {
                        recurring: property::Recurring {
                            pattern: from_do_cur::recur::strprecur("every Mon").unwrap(),
                            ts: None,
                        },
                        begin: None,
                        until: Some(property::Due {
                            rel: None,
                            ts: Some(ts(until_ts)),
                        }),
                    },
                }),
            ],
        };
        assert_eval_ok(
            input,
            Program {
                blocks: vec![
                    Block::Directive(Directive::Now(directive::Now { now: ts(now) })),
                    Block::ToDo(ToDo {
                        t: ToDoType::ToDo,
                        head: ss("FromDo"),
                        body: None,
                        schedule: Schedule::Recurring {
                            recurring: property::Recurring {
                                pattern: from_do_cur::recur::strprecur("every Mon").unwrap(),
                                ts: Some(ts(last)),
                            },
                            begin: None,
                            until: Some(due_ts_ref(until_ts, now)),
                        },
                    }),
                    child_block("FromDo", "2026-04-13T08:00:00+00:00[UTC]", now),
                    child_block("FromDo", last, now),
                ],
            },
        );
    }

    #[test]
    fn todo_recurring_with_ts_resumes() {
        //| :now 2026-04-08T08:00:00+00:00[UTC]
        //| :ahead 2
        //|
        //| -	FromDo
        //| 	recurring every Mon
        //| 		2026-04-13T08:00:00+00:00[UTC]
        let now = "2026-04-08T08:00:00+00:00[UTC]";
        let prior_last = "2026-04-13T08:00:00+00:00[UTC]";
        let new_last = "2026-04-27T08:00:00+00:00[UTC]";
        let input = Program {
            blocks: vec![
                Block::Directive(Directive::Now(directive::Now { now: ts(now) })),
                Block::Directive(Directive::Ahead(directive::Ahead { ahead: 2 })),
                Block::ToDo(ToDo {
                    t: ToDoType::ToDo,
                    head: ss("FromDo"),
                    body: None,
                    schedule: Schedule::Recurring {
                        recurring: property::Recurring {
                            pattern: from_do_cur::recur::strprecur("every Mon").unwrap(),
                            ts: Some(ts(prior_last)),
                        },
                        begin: None,
                        until: None,
                    },
                }),
            ],
        };
        assert_eval_ok(
            input,
            Program {
                blocks: vec![
                    Block::Directive(Directive::Now(directive::Now { now: ts(now) })),
                    Block::Directive(Directive::Ahead(directive::Ahead { ahead: 2 })),
                    Block::ToDo(ToDo {
                        t: ToDoType::ToDo,
                        head: ss("FromDo"),
                        body: None,
                        schedule: Schedule::Recurring {
                            recurring: property::Recurring {
                                pattern: from_do_cur::recur::strprecur("every Mon").unwrap(),
                                ts: Some(ts(new_last)),
                            },
                            begin: None,
                            until: None,
                        },
                    }),
                    child_block("FromDo", "2026-04-20T08:00:00+00:00[UTC]", now),
                    child_block("FromDo", new_last, now),
                ],
            },
        );
    }

    #[test]
    fn todo_recurring_not_to_do_no_generation() {
        //| :now 2026-04-08T08:00:00+00:00[UTC]
        //|
        //| +	FromDo
        //| 	recurring every Mon
        let now = "2026-04-08T08:00:00+00:00[UTC]";
        let input = Program {
            blocks: vec![
                Block::Directive(Directive::Now(directive::Now { now: ts(now) })),
                Block::ToDo(ToDo {
                    t: ToDoType::NotToDo,
                    head: ss("FromDo"),
                    body: None,
                    schedule: recurring_schedule("every Mon"),
                }),
            ],
        };
        assert_eval_ok(input.clone(), input);
    }

    #[test]
    fn todo_recurring_body_duplicated_to_children() {
        //| :now 2026-04-08T08:00:00+00:00[UTC]
        //| :ahead 1
        //|
        //| -	FromDo
        //| 	recurring every Mon
        //|
        //| 	What's the buzz?
        let now = "2026-04-08T08:00:00+00:00[UTC]";
        let last = "2026-04-13T08:00:00+00:00[UTC]";
        let body = SString {
            span: Span { lo: 0, hi: 17 },
            node: "What's the buzz?\n".to_string(),
        };
        let input = Program {
            blocks: vec![
                Block::Directive(Directive::Now(directive::Now { now: ts(now) })),
                Block::Directive(Directive::Ahead(directive::Ahead { ahead: 1 })),
                Block::ToDo(ToDo {
                    t: ToDoType::ToDo,
                    head: ss("FromDo"),
                    body: Some(body.clone()),
                    schedule: recurring_schedule("every Mon"),
                }),
            ],
        };
        assert_eval_ok(
            input,
            Program {
                blocks: vec![
                    Block::Directive(Directive::Now(directive::Now { now: ts(now) })),
                    Block::Directive(Directive::Ahead(directive::Ahead { ahead: 1 })),
                    Block::ToDo(ToDo {
                        t: ToDoType::ToDo,
                        head: ss("FromDo"),
                        body: Some(body.clone()),
                        schedule: Schedule::Recurring {
                            recurring: property::Recurring {
                                pattern: from_do_cur::recur::strprecur("every Mon").unwrap(),
                                ts: Some(ts(last)),
                            },
                            begin: None,
                            until: None,
                        },
                    }),
                    Block::ToDo(ToDo {
                        t: ToDoType::ToDo,
                        head: ss("FromDo"),
                        body: Some(body),
                        schedule: Schedule::Once {
                            due: Some(due_ts_ref(last, now)),
                            late_due: None,
                        },
                    }),
                ],
            },
        );
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
