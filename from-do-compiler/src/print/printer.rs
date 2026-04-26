use from_do_cur::cur::*;

use crate::parse::*;

pub struct Printer {
    buffer: String,
}

impl Printer {
    pub fn new() -> Self {
        Self {
            buffer: String::new(),
        }
    }

    pub fn print(&mut self, program: &Program) -> String {
        self.buffer = String::new();

        for block in &program.blocks {
            match block {
                Block::Error(_) => continue,
                Block::Directive(directive) => match directive {
                    Directive::Now(directive) => {
                        self.buffer.push_str(&format!(":now {:#}\n", directive.now));
                    }
                    Directive::Tz(directive) => {
                        self.buffer.push_str(&format!(
                            ":tz {:#}\n",
                            directive
                                .tz
                                .iana_name()
                                .expect("A parsed tz should have an IANA name")
                        ));
                    }
                },
                Block::ToDo(todo) => {
                    let head_marker = match todo.t {
                        ToDoType::ToDo => "-",
                        ToDoType::NotToDo => "+",
                    };
                    self.buffer
                        .push_str(&format!("{}\t{}\n", head_marker, todo.head.node));
                    if let Some(due) = &todo.due {
                        let rel_str = if let Some(rel) = &due.rel {
                            &strfcur(rel)
                        } else {
                            ""
                        };
                        let ts_str = if let Some(ts) = &due.ts {
                            &format!("{:#}", ts)
                        } else {
                            panic!("invalid due property: ts is None");
                        };
                        self.buffer
                            .push_str(&format!("\tdue {}\n\t\t{}\n", rel_str, ts_str));
                    }
                    if let Some(late_due) = &todo.late_due {
                        let rel_str = if let Some(rel) = &late_due.rel {
                            &strfcur(rel)
                        } else {
                            ""
                        };
                        let ts_str = if let Some(ts) = &late_due.ts {
                            &format!("{:#}", ts)
                        } else {
                            panic!("invalid due property: ts is None");
                        };
                        self.buffer
                            .push_str(&format!("\tlate due {}\n\t\t{}\n", rel_str, ts_str));
                    }
                    self.buffer.push_str("\t\n");
                    if let Some(body) = &todo.body {
                        for line in body.node.lines() {
                            self.buffer.push_str(&format!("\t{}\n", line));
                        }
                    }
                }
            }
            self.buffer.push_str("\n");
        }
        self.buffer.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lex::*;
    use indoc::indoc;
    use pretty_assertions::assert_eq;

    fn assert_print(input: Program, expected: &str) {
        let mut printer = super::Printer::new();
        let actual = printer.print(&input);
        assert_eq!(actual, expected);
    }

    // ! Note: Span is not important for printing.
    const SPAN: Span = Span { lo: 0, hi: 0 };

    fn s(node: &str) -> SString {
        SString {
            span: SPAN,
            node: node.to_string(),
        }
    }

    fn ts(value: &str) -> jiff::Zoned {
        value.parse().unwrap()
    }

    fn tz(name: &str) -> jiff::tz::TimeZone {
        jiff::tz::TimeZone::get(name).unwrap()
    }

    #[test]
    fn sanity_0() {
        // empty program
        assert_print(Program { blocks: vec![] }, "");
    }

    #[test]
    fn sanity_1() {
        //| :now 2026-04-08T08:00:00+00:00[UTC]
        //|
        //| -	FromDo
        assert_print(
            Program {
                blocks: vec![
                    Block::Directive(Directive::Now(directive::Now {
                        now: ts("2026-04-08T08:00:00+00:00[UTC]"),
                    })),
                    Block::ToDo(ToDo {
                        t: ToDoType::ToDo,
                        head: s("FromDo"),
                        body: None,
                        due: None,
                        late_due: None,
                    }),
                ],
            },
            indoc! {"
                :now 2026-04-08T08:00:00+00:00[UTC]
                
                -	FromDo
                	
                
            "},
        );
    }

    #[test]
    fn sanity_2() {
        //| :now 2026-04-08T08:00:00+00:00[UTC]
        //|
        //| :now 2026-04-01T08:00:00+00:00[UTC]
        //|
        //| -	Hello, FromDo!
        //| 	due
        //| 		2026-04-08T12:00:00+00:00[UTC]
        //| 	late due
        //| 		2026-04-09T12:00:00+00:00[UTC]
        //|
        //| 	What's the buzz?
        //| 	Tell me what's-a-happening
        //|
        //| 	Why should you want to know?
        //| 	Don't you mind about the future
        //| 	Think about today instead
        //|
        //| -	FromDo
        //|
        //| 	Let me try to cool down your face a bit
        //| 	That feels nice, so nice
        //|
        //| 	Mary, that is good
        //| 	What I need right here and now
        assert_print(
            Program {
                blocks: vec![
                    Block::Directive(Directive::Now(directive::Now {
                        now: ts("2026-04-08T08:00:00+00:00[UTC]"),
                    })),
                    Block::Directive(Directive::Now(directive::Now {
                        now: ts("2026-04-01T08:00:00+00:00[UTC]"),
                    })),
                    Block::ToDo(ToDo {
                        t: ToDoType::ToDo,
                        head: s("Hello, FromDo!"),
                        body: Some(s(indoc! {"
                            What's the buzz?
                            Tell me what's-a-happening

                            Why should you want to know?
                            Don't you mind about the future
                            Think about today instead
                        "})),
                        due: Some(property::Due {
                            rel: None,
                            ts: Some(ts("2026-04-08T12:00:00+00:00[UTC]")),
                        }),
                        late_due: Some(property::Due {
                            rel: None,
                            ts: Some(ts("2026-04-09T12:00:00+00:00[UTC]")),
                        }),
                    }),
                    Block::ToDo(ToDo {
                        t: ToDoType::ToDo,
                        head: s("FromDo"),
                        body: Some(s(indoc! {"
                            Let me try to cool down your face a bit
                            That feels nice, so nice

                            Mary, that is good
                            What I need right here and now
                        "})),
                        due: None,
                        late_due: None,
                    }),
                ],
            },
            indoc! {"
                :now 2026-04-08T08:00:00+00:00[UTC]
                
                :now 2026-04-01T08:00:00+00:00[UTC]
                
                -	Hello, FromDo!
                	due 
                		2026-04-08T12:00:00+00:00[UTC]
                	late due 
                		2026-04-09T12:00:00+00:00[UTC]
                	
                	What's the buzz?
                	Tell me what's-a-happening
                	
                	Why should you want to know?
                	Don't you mind about the future
                	Think about today instead
                
                -	FromDo
                	
                	Let me try to cool down your face a bit
                	That feels nice, so nice
                	
                	Mary, that is good
                	What I need right here and now
                
            "},
        );
    }

    #[test]
    fn block_error_ignored() {
        // Block::Error(LexerError("What's the buzz?")) is skipped.
        //| :now 2026-04-08T08:00:00+00:00[UTC]
        //|
        //| -	FromDo
        assert_print(
            Program {
                blocks: vec![
                    Block::Directive(Directive::Now(directive::Now {
                        now: ts("2026-04-08T08:00:00+00:00[UTC]"),
                    })),
                    Block::Error(Error::LexerError(s("What's the buzz?"))),
                    Block::ToDo(ToDo {
                        t: ToDoType::ToDo,
                        head: s("FromDo"),
                        body: None,
                        due: None,
                        late_due: None,
                    }),
                ],
            },
            indoc! {"
                :now 2026-04-08T08:00:00+00:00[UTC]
                
                -	FromDo
                	
                
            "},
        );
    }

    #[test]
    fn directive_now() {
        //| :now 2026-04-08T08:00:00+00:00[UTC]
        assert_print(
            Program {
                blocks: vec![Block::Directive(Directive::Now(directive::Now {
                    now: ts("2026-04-08T08:00:00+00:00[UTC]"),
                }))],
            },
            indoc! {"
                :now 2026-04-08T08:00:00+00:00[UTC]
                
            "},
        );
    }

    #[test]
    fn directive_tz() {
        //| :tz America/New_York
        assert_print(
            Program {
                blocks: vec![Block::Directive(Directive::Tz(directive::Tz {
                    tz: tz("America/New_York"),
                }))],
            },
            indoc! {"
                :tz America/New_York
                
            "},
        );
    }

    #[test]
    fn todo_simple() {
        //| -	FromDo
        assert_print(
            Program {
                blocks: vec![Block::ToDo(ToDo {
                    t: ToDoType::ToDo,
                    head: s("FromDo"),
                    body: None,
                    due: None,
                    late_due: None,
                })],
            },
            indoc! {"
                -	FromDo
                	
                
            "},
        );
    }

    #[test]
    fn todo_simple_not() {
        //| +	FromDo
        assert_print(
            Program {
                blocks: vec![Block::ToDo(ToDo {
                    t: ToDoType::NotToDo,
                    head: s("FromDo"),
                    body: None,
                    due: None,
                    late_due: None,
                })],
            },
            indoc! {"
                +	FromDo
                	
                
            "},
        );
    }

    #[test]
    fn todo_due() {
        //| -	Hello, FromDo!
        //| 	due
        //| 		2026-04-08T12:00:00+00:00[UTC]
        assert_print(
            Program {
                blocks: vec![Block::ToDo(ToDo {
                    t: ToDoType::ToDo,
                    head: s("Hello, FromDo!"),
                    body: None,
                    due: Some(property::Due {
                        rel: None,
                        ts: Some(ts("2026-04-08T12:00:00+00:00[UTC]")),
                    }),
                    late_due: None,
                })],
            },
            indoc! {"
                -	Hello, FromDo!
                	due 
                		2026-04-08T12:00:00+00:00[UTC]
                	
                
            "},
        );
    }

    #[test]
    fn todo_body_3() {
        //| -	FromDo
        //|
        //| 	Why should you want to know?
        //| 	Don't you mind about the future
        //| 	Think about today instead
        assert_print(
            Program {
                blocks: vec![Block::ToDo(ToDo {
                    t: ToDoType::ToDo,
                    head: s("FromDo"),
                    body: Some(s(indoc! {"
                        Why should you want to know?
                        Don't you mind about the future
                        Think about today instead
                    "})),
                    due: None,
                    late_due: None,
                })],
            },
            indoc! {"
                -	FromDo
                	
                	Why should you want to know?
                	Don't you mind about the future
                	Think about today instead
                
            "},
        );
    }

    #[test]
    fn todo_1() {
        //| -	Hello, FromDo!
        //| 	due
        //| 		2026-04-08T12:00:00+00:00[UTC]
        //|
        //| 	What's the buzz?
        //| 	Tell me what's-a-happening
        //|
        //| 	Why should you want to know?
        //| 	Don't you mind about the future
        assert_print(
            Program {
                blocks: vec![Block::ToDo(ToDo {
                    t: ToDoType::ToDo,
                    head: s("Hello, FromDo!"),
                    body: Some(s(indoc! {"
                        What's the buzz?
                        Tell me what's-a-happening

                        Why should you want to know?
                        Don't you mind about the future
                    "})),
                    due: Some(property::Due {
                        rel: None,
                        ts: Some(ts("2026-04-08T12:00:00+00:00[UTC]")),
                    }),
                    late_due: None,
                })],
            },
            indoc! {"
                -	Hello, FromDo!
                	due 
                		2026-04-08T12:00:00+00:00[UTC]
                	
                	What's the buzz?
                	Tell me what's-a-happening
                	
                	Why should you want to know?
                	Don't you mind about the future
                
            "},
        );
    }
}
