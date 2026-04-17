use crate::parse::*;

pub struct Evaluator {
    context: Context,
}

pub struct Context {
    // TODO: parent: Option<Box<Context>>,
    now: jiff::Timestamp,
}

impl Evaluator {
    pub fn new() -> Self {
        Self {
            context: Context {
                // TODO: parent: None,
                now: jiff::Timestamp::now(),
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
                    context.now = now.now;
                    Ok(block.clone())
                }
            },
            Block::ToDo(todo) => {
                let mut target = todo.clone();
                if let Some(due) = &todo.due {
                    target.due_in = Some(context.now.duration_until(*due));
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

    fn assert_eval(input: Program, expected: Program) {
        assert_eq!(Evaluator::new().eval(&input), Ok(expected));
    }

    #[test]
    fn test_eval() {
        let input = Program {
            blocks: vec![
                Block::Directive(Directive::Now(directive::Now {
                    now: "2026-04-08T08:00:00Z".parse().unwrap(),
                })),
                Block::ToDo(ToDo {
                    head: SString {
                        span: Span { lo: 1, hi: 5 },
                        node: "test".to_string(),
                    },
                    body: None,
                    due: Some("2026-04-08T12:00:00Z".parse().unwrap()),
                    due_in: None,
                }),
            ],
        };
        assert_eval(
            input,
            Program {
                blocks: vec![
                    Block::Directive(Directive::Now(directive::Now {
                        now: "2026-04-08T08:00:00Z".parse().unwrap(),
                    })),
                    Block::ToDo(ToDo {
                        head: SString {
                            span: Span { lo: 1, hi: 5 },
                            node: "test".to_string(),
                        },
                        body: None,
                        due: Some("2026-04-08T12:00:00Z".parse().unwrap()),
                        due_in: Some(jiff::SignedDuration::from_hours(4)),
                    }),
                ],
            },
        );
    }
}
