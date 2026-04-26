# Recur Grammar

This document defines the canonical recurrence pattern grammar and a English recurrence grammar.

The current grammar covers only date recurrence.

## Pattern

A (recurrence) pattern is a phrase that defines a set of calendar dates given a reference date. A pattern may constrain none, one, or more of the following fields:

- day-of-week or day-of-month
- month
- year

The canonical form of a pattern that uses the following syntax:

```text
(<day-of-week> | <day-of-month>) <month> <year>
```

`<day-of-week>` is either `Mon`, `Tue`, `Wed`, `Thu`, `Fri`, `Sat`, or `Sun`. `<day-of-month>` is a number from `1` to `31`. `<month>` is either `Jan`, `Feb`, `Mar`, `Apr`, `May`, `Jun`, `Jul`, `Aug`, `Sep`, `Oct`, `Nov`, `Dec`, or a number from `1` to `12`. `<year>` is a from `1` to `9999`. For `<day-of-month>`, negative numbers mean counting from the last value, so `-1` in `<day-of-month>` means the last day of the month. A wildcard `*` means the full range of values for a field.

The literals, numbers, and wildcards defined above are called "atoms".

Besides atoms, a field also accept a range of atoms in the syntax of `A-B`, meaning the set of values from `A` to `B` inclusive, and a stepped range in the syntax of `A-B/C`, meaning the set of values from `A` to `B` inclusive that are congruent to `A` modulo `C`. Note that literals also have an implicit numeric value, so `Mon-Sun` and `Jan-Dec` are also ranges. A syntactic sugar for a stepped range is `*/C`, which means `A-B/C` where `A` is the first value of that field and `B` is the last value of that field. If any range exceeds the field's range, it is clamped to the field's range.

Specifically for the day field, a wildcard `*` is ambiguous between day-of-week and day-of-month, but both have the same meaning of "every day" unambiguously. However, a stepped range with wildcard is ambiguous between day-of-week and day-of-month, so it isn't accepted, and should be explicitly written as `Mon-Sun/C` for day-of-week or `1-31/C` for day-of-month.

Besides atoms and ranges, a field also accepts a list of atoms and/or ranges separated by `,`, meaning the union of the sets defined by each value.

### Normalization

A normalized pattern is a pattern that is normalized according to the following rules:

- a range `A-A` is normalized to `A`
- a range `A-B/1` is normalized to `A-B`
- a range that covers the full field is normalized to `*`
- a list including `*` is normalized to `*`
- a list including duplicates is normalized without duplicates
- a list is sorted in ascending order of atoms, or the LHS atom of a range

## English

A pattern can be formatted into a English representation and a English representation can be parsed into a pattern. The English grammar is designed to be human-friendly and unambiguous, and to have a one-to-one mapping with the canonical pattern form.

A English representation has the following shape:

- a day-of-week or a day-of-month fragment
- a month fragment
- a year fragment

### Day Fragments

A day fragment is either a wildcard, a day-of-week fragment, or a day-of-month fragment.

- `every day` corresponds to a wildcard atom

#### Day-of-Week Fragments

A day-of-week fragment constrains the weekday field and has the following forms:

- `every <weekday>` corresponds to a non-wildcard atom
- `every day in <weekday>-<weekday>` corresponds to a range
- `every <ordinal> day in <weekday>-<weekday>` corresponds to a stepped range

`<weekday>` can be `Mon`, `Tue`, `Wed`, `Thu`, `Fri`, `Sat`, or `Sun` in formatting, and can be any prefix of their full names in parsing. `<ordinal>` can be `1st`, `2nd`, `3rd`, `4th`, etc. in formatting, and can be a number followed by `st`, `nd`, `rd`, or `th` in parsing.

If the pattern field is a list, the English form preserves the first `every`, and joins the rest of the items, in the following forms:

- `every <A> and <B>` for two items
- `every <A>, <B>, ..., and <C>` for more items

#### Day-of-Month Fragments

A day-of-month fragment constrains the day-of-month field and has the following forms:

- `every <day>` corresponds to a non-wildcard atom
- `every day in <day>-<day>` corresponds to a range
- `every <ordinal> day in <day>-<day>` corresponds to a stepped range

`<day>` can be an ordinal from `1st` to `31st`, or `last` to `31st last` in formatting, and can be any number followed by `st`, `nd`, `rd`, or `th`, optionally followed by a `last` in parsing. `<ordinal>` can be `1st`, `2nd`, `3rd`, `4th`, etc. in formatting, and can be a number followed by `st`, `nd`, `rd`, or `th` in parsing.

If the pattern field is a list, the English form preserves the first `every`, and joins the rest of the items, in the following forms:

- `every <A> and <B>` for two items
- `every <A>, <B>, ..., and <C>` for more items

### Month Fragments

A month fragment constrains the month field and has the following forms:

- `every <month>` corresponds to a non-wildcard atom
- `every month` corresponds to a wildcard atom
- `every month in <month>-<month>` corresponds to a range
- `every <ordinal> month in <month>-<month>` corresponds to a stepped range
- `every <ordinal> month` corresponds to a stepped range with wildcard

`<month>` can be `Jan`, `Feb`, `Mar`, `Apr`, `May`, `Jun`, `Jul`, `Aug`, `Sep`, `Oct`, `Nov`, or `Dec` in formatting, and can be any prefix of their full names in parsing. `<ordinal>` can be `1st`, `2nd`, `3rd`, etc. in formatting, and can be a number followed by `st`, `nd`, `rd`, or `th` in parsing.

If the pattern field is a list, the English form preserves the first `every`, and joins the rest of the items, in the following forms:

- `every <A> and <B>` for two items
- `every <A>, <B>, ..., and <C>` for more items

### Year Fragments

A year fragment constrains the year field and has the following forms:

- `<year>` corresponds to a non-wildcard atom
- `every year` corresponds to a wildcard atom
- `every year in <year>-<year>` corresponds to a range
- `every <ordinal> year in <year>-<year>` corresponds to a stepped range
- `every <ordinal> year` corresponds to a stepped range with wildcard

`<year>` can be a number from `1` to `9999` in formatting, and can be any number from `1` to `9999` in parsing. `<ordinal>` can be `1st`, `2nd`, `3rd`, etc. in formatting, and can be a number followed by `st`, `nd`, `rd`, or `th` in parsing.

### Fragment Concatenation

The fragments are concatenated in the order of day-of-week or day-of-month, then month, and then year, joined by a space, a literal `in`, and a space.

- The concatenation word(s) preceding month can be simplified. If the month fragment is a non-wildcard atom, the `every` in the month fragment can be omitted (as well as the space between it; same for all followings). If the month fragment is a wildcard atom or a range, the `in every month` can be omitted. Otherwise, the concatenation word(s) cannot be simplified.

  ```text
  every day in every Jan -> every day in Jan
  every day in every month -> every day
  every day in every month in Jan-Jun -> every day in Jan-Jun
  every day in every 2nd month from Jan to Jun -> every day in every 2nd month in Jan-Jun
  every day in every 2nd month -> every day in every 2nd month
  ```

  We are using "every day" as the example here. But the same rule applies for other cases.

- Similar simplification applies to the concatenation word(s) preceding year. If the year fragment is a wildcard atom or a range, the `in every year` can be omitted. Otherwise, the concatenation word(s) cannot be simplified.

  ```text
  ... every month in 2026 -> ... every month in 2026
  ... every month in every year -> ... every month
  ... every month in every year in 2026-2030 -> ... every month in 2026-2030
  ... every month in every 2nd year from 2026-2030 -> every month in every 2nd year in 2026-2030
  ... every month in every 2nd year -> every month in every 2nd year
  ```

  We are using "every month" as the example here. But the same rule applies for other cases, and even if the month fragment is omitted.

Specifically for parsing, all concatenation word(s) between fragments, including `in`, `every`, `month`, and `year`, are optional, but if any of them is present, the order of fragments must be preserved.

## Parsing and Formatting

It is expected that a pattern can be formatted into a English representation and a English representation can be parsed into a pattern, and the two operations are inverses of each other. However, the parsing operation is more lenient than the formatting operation, so that it can accept more variations of the English representation, as long as they are unambiguous.

As a general rule, the parsing operation is case insensitive and whitespace insensitive. The specific rules are already described in the sections above.

For parsing, the list separator is lenient: if a comma is present, all surrounding whitespace and the literal `and` are optional (the comma alone is enough to separate items); if no comma is present, `and` (with surrounding whitespace) is required.
