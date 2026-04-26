---
description: "Use when creating, adapting, or verifying Rust tests in from-do-compiler, especially parser-style compiler tests. Covers semantic grouping, test naming, human-readable comment blocks, span expectations, test completeness and flow-path coverage, and preferring Result::Err coverage over panic tests."
name: "Compiler Test Style"
applyTo: "from-do-compiler/src/**/*.rs"
---
# Compiler Test Style

## Reference

- Use from-do-compiler/src/parse/parser.rs as the primary style reference for compiler crate tests.

## Organization

- Group tests by semantics.
- Start with `sanity_*` tests that cover the simplest end-to-end cases.
- Follow with unit-like groups organized by subject, such as `block_*`, `directive_*`, and `todo_*`.
- Within each subject, progress from the simplest cases to focused specific cases and then to comprehensive scenarios.

## Naming

- Use names like `todo_simple` for the simplest focused case.
- Use names like `todo_body_1` or `todo_out_4` for specific focused variants.
- For specific cases, the numeric suffix is ad hoc and often reflects the number of lines or examples in the fixture.
- Use names like `todo_1` and `todo_2` for comprehensive scenarios. In those cases, the suffix acts as an index rather than a semantic label.

## Fixture Presentation

- Put a human-readable comment block at the top of every test case.
- Default to the source-like fixture layout used in parser.rs.
- Prefix only the source-like fixture lines of that top comment block with `//| ` so tabs render and align consistently in Rust source.
- Use plain `// ` comments for descriptive lines in that block, such as `empty input` or AST-oriented labels.
- If there is a escaped tab `\t` in the fixture code, render it as a real tab in the comment block.
- Add AST-oriented context when it clarifies the intent better than the raw fixture alone.
- For simple fixtures, prefer project-meaningful content such as `FromDo`, `Hello, FromDo!`, and `2026-04-08T08:00:00Z`. This is not a hard rule, but it helps make the tests more engaging and memorable.
- For richer fixtures, prefer lyrics or other human-readable prose; recurring lines from `What's the Buzz` are a good fit for this codebase. Choose the contiguous lines. If you choose a line from a block, the entire block should be in the fixture.
- Keep the fixture explicit in the test body rather than hiding it behind additional builders beyond the local helpers already present in the module.
- Use `indoc!` macro for multi-line fixtures.
- Preserve local helper conventions when a module already has them, such as `t!`, `ts`, `with_span`, and `assert_program` in parser tests.

The lyrics of "What's the Buzz" is as follows:

```
["What's The Buzz"]

[APOSTLES]
What's the buzz?
Tell me what's-a-happening

[JESUS]
Why should you want to know?
Don't you mind about the future
Don't you try to think ahead
Save tomorrow for tomorrow
Think about today instead

[APOSTLES]
What's the buzz?
Tell me what's-a-happening

[JESUS]
I could give you facts and figures
I could give you plans and forecasts
Even tell you where I'm going -

[APOSTLES]
When do we ride into Jerusalem?
You might also like
Heaven on Their Minds
Murray Head
This Jesus Must Die
Jesus Christ Superstar Original Studio Cast
Damned for All Time/Blood Money
Jesus Christ Superstar Original Studio Cast
[JESUS]
Why should you want to know?
Why are you obsessed with fighting
Times and fates you can't defy?
If you knew the path we're riding
You'd understand it less than I

[APOSTLES]
What's the buzz?
Tell me what's-a-happening

[MARY MAGDALENE]
Let me try to cool down your face a bit

[JESUS]
That feels nice, so nice
Mary, that is good
While you prattle through your supper
Where and when and who and how
She alone has tried to give me
What I need right here and now

[APOSTLES]
What's the buzz?
Tell me what's-a-happening
["Strange Thing Mystifying"]

[JUDAS]
It seems to me a strange thing, mystifying
That a man like you can waste his time
On women of her kind
Yes, I can understand that she amuses
But to let her stroke you, kiss your hair
Is hardly in your line
It's not that I object to her profession
But she doesn't fit in well with what you teach and say
It doesn't help us if you're inconsistent
They only need a small excuse to put us all away

[JESUS]
Who are you to criticize her?
Who are you to despise her?
Leave her, leave her, let her be now
Leave her, leave her, she's with me now
If your slate is clean, then you can throw stones
If your slate is not, then leave her alone
I'm amazed that men like you
Can be so shallow, thick and slow
There is not a man among you
Who knows or cares if I come or go
[ALL except JUDAS]
No, you're wrong! You're very wrong!
How can you say that!

[JESUS]
Not one, not one of you!
```

## Assertions

- Treat semantic structure as the primary assertion target.
- Update spans when fixture changes require it, but do not create span-focused tests unless the change is specifically about span calculation.
- Prefer test cases that exercise `Result::Err` paths. Do not add panic-based tests when the panic indicates a broken precondition or an internal compiler error.

## Adapting Tests

- When asked to adapt the tests, assume the implementation is usually correct and update test fixtures and expectations first.
- If the implementation appears inconsistent with the surrounding semantics, fix the implementation only when the evidence is clear.
- If the intended semantics remain ambiguous, stop and ask the user before making speculative changes.

## Verifying Tests

- When asked to verify the tests, run the relevant test suite, adapt the tests to the current semantics, and normalize formatting, grouping, naming, and comment blocks to this style.
- Verifying the tests also includes checking completeness against the current semantics and control flow. Ask whether all meaningful flow paths are covered, including success paths, focused edge cases, and user-visible error paths.
- Prefer minimal, focused test changes that preserve the established organization.