---
description: "Use when creating, adapting, or verifying tests. Covers concise but comprehensive test design, matrix-driven coverage, direct fixtures, descriptive names, edge cases, and roundtrip or inverse checks if available."
name: "Test Style"
---
# Test Style

## Goal

- Create tests that are concise but comprehensive in coverage.
- Make it obvious what behavior is being tested without forcing the reader to chase helpers.

## Coverage Planning

- Derive the test matrix from the governing spec.
- Cover the simplest cases first, then the meaningful combination space, then edge cases and precedence rules.
- If behavior is fundamentally combinatorial, do not stop at a few spot checks.
- If operation comes in pair, i.e., forward and backward, test both directions explicitly.
- Include edge cases that commonly drift.

## Test Shape

- For simple one-case tests, define direct assertions.
- For broader tests, define an explicit case matrix and iterate over it in a loop.
- A matrix row is self-contained: include the relevant inputs and expected outputs in the row itself.
- Prefer the simplest direct constructor or literal for the value under test instead of indirect setup chains.
- Prefer direct construction of the exact type being asserted instead of constructing a larger wrapper and extracting part of it.
- Inline local fixtures that would otherwise force the reader to jump to a helper to understand the case.
- Don't use helper functions that hide what is being asserted. 
- Use helpers that removes obvious repetition without obscuring the behavior.

## Naming

- Use precise, behavior-oriented test names that state the unit and case.
- Prefer names that are like `<operation>_<specific-model-shape>` or `<operation>_<specific-model-shape>_<specific-condition>`.
- When a test is matrix-driven, name the test after the behavior family it covers.

## Assertions

- If a behavior has both broad and narrow surfaces, test the lower-level unit directly and test the higher-level canonical behavior separately.
