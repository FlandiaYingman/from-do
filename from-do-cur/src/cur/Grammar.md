# Cur Grammar

This document describes the English timestamp surface used by the `cur` parser and formatter.

## Reference Model

Every parse or format operation is evaluated relative to a reference datetime.

The reference datetime is a `jiff::Zoned` value, so it already carries:

- the reference timestamp
- the effective local civil date and time
- the timezone used for relative interpretation

Relative clauses are interpreted in the timezone of the reference datetime.

Unless otherwise stated, examples in this document assume the reference datetime is `2026-04-07T16:42:00Z`, i.e., Tuesday 7 Apr 2026 at 16:42:00 in UTC.

## Phrase

A `cur` Phrase is either:

- a date spec clause and a time spec clause
- a calendar offset

Examples of date spec clause and time spec clause:

- `today` (date spec clause only)
- `tomorrow at 09` (date spec clause and time spec clause)
- `this Monday at 9 AM` (date spec clause and time spec clause)
- `next Friday at 17:30` (date spec clause and time spec clause)
- `on 8 Apr 2026` (date spec clause only)
- `on 8 Apr 2026 at 9:30 PM` (date spec clause and time spec clause)
- `at 9:30 PM` (time spec clause only)

Either a date spec clause or a time spec clause may be absent (not both). If one of them is absent, the phrase inherits the corresponding date or time fields from the reference datetime.

Examples of calendar offset:

- `in 3 days`
- `2 weeks ago`
- `in 1 month`
- `3 years ago`

## Date Spec and Time Spec Clauses

### Relative Date Spec Clauses

A relative date spec clause is a clause that determines a date by applying a calendar offset to the reference date.

#### Yesterday, Today, and Tomorrow

- `yesterday` applies a calendar offset of `-1` day to the reference date.
- `today` applies a calendar offset of `0` days to the reference date.
- `tomorrow` applies a calendar offset of `+1` day to the reference date.

For example, assuming the reference datetime `2026-04-07T16:42:00Z`:

- `yesterday` means `2026-04-06`
- `today` means `2026-04-07`
- `tomorrow` means `2026-04-08`

#### Weekdays

A weekday is of the shape `<relation> <weekday>`. The relation is one of `last`, `this`, or `next`. The weekday is one of `Monday` through `Sunday`.

- `last` applies a calendar offset of `-1` week to the reference date.
- `this` applies a calendar offset of `0` weeks to the reference date.
- `next` applies a calendar offset of `+1` week to the reference date.
- `Monday` through `Sunday` applies an appropriate day offset to the reference date to get to the specified weekday.

Note that Monday begins the week and Sunday ends the week. This follows ISO week semantics.

For example, assuming the reference datetime `2026-04-07T16:42:00Z` (Tuesday):

- `this Monday` means `2026-04-06`
- `this Sunday` means `2026-04-12`
- `next Monday` means `2026-04-13`
- `last Monday` means `2026-03-30`

### Absolute Date Spec Clauses

An absolute date spec clause is a clause that determines a date by directly specifying calendar fields, i.e., day, month, and year.

The canonical date phrase is of the shape `on <d> <m> <y>`.

- `d` is a numeric day-of-month without preceding 0's and without an ordinal suffix.
- `m` is a short English month name: `Jan` through `Dec`.
- `y` is a four-digit year.

### Time Spec Clauses

A time spec clause is an absolute clause that determines a time by directly specifying clock fields, i.e., hour, minute, and second.

The canonical time phrase is of the shape as follows.

### 24-Hour Forms

- `at HH`
- `at HH:MM`
- `at HH:MM:SS`

Examples:

- `at 09`
- `at 09:30`
- `at 09:30:45`

Canonical 24-hour formatting uses

- two digits for all numeric components.

### 12-Hour Forms

- `at H AM`
- `at H:MM AM`
- `at H:MM:SS AM`
- `at H PM`
- `at H:MM PM`
- `at H:MM:SS PM`

Examples:

- `at 9 AM`
- `at 9:30 AM`
- `at 9:30:45 PM`

Canonical 12-hour formatting uses:

- no leading zero on the hour
- two digits for minute, if present
- two digits for second, if present
- uppercase `AM` and `PM`

### Omission Rules

In formatting and parsing, the following omission rules apply.

- if minute and second are both `0`, only the hour is present.
- if second is `0` and minute is non-zero, hour and minute are present.
- otherwise hour, minute, and second are present.

Examples:

- `09:00:00` formats as `at 09` in 24-hour mode
- `09:30:00` formats as `at 09:30` in 24-hour mode
- `09:30:45` formats as `at 09:30:45` in 24-hour mode
- `21:00:00` formats as `at 9 PM` in 12-hour mode
- `21:30:00` formats as `at 9:30 PM` in 12-hour mode
- `21:30:45` formats as `at 9:30:45 PM` in 12-hour mode

The parsing is analogous.

## Calendar Offsets

A calendar offset is of either of the forms:

- `in <n> <unit>`
- `<n> <unit> ago`

In both forms, `n` is a natural number, and `unit` is one of `d`, `day`, `days`, `w`, `week`, `weeks`, `month`, `months`, `year`, or `years`. The formatter determines the singular or plural form of the unit based on `n`. The parser accepts both singular and plural forms of the unit regardless of `n`. The formatter always use the long form of the unit (i.e., no `d` and `w`). The parser accepts both the short and long forms of the unit.

The semantics of a calendar offset is to apply the specified offset to the reference date, and then clamp the result to a valid civil date if necessary. `in` applies a positive offset, and `ago` applies a negative offset.

For example, assuming the reference datetime `2026-04-07T16:42:00Z`:

- `in 3 days` means `2026-04-10T16:42:00Z`
- `3 days ago` means `2026-04-04T16:42:00Z`
- `in 2 weeks` means `2026-04-21T16:42:00Z`
- `2 weeks ago` means `2026-03-24T16:42:00Z`
- `in 1 month` means `2026-05-07T16:42:00Z`
- `1 month ago` means `2026-03-07T16:42:00Z`
- `in 1 year` means `2027-04-07T16:42:00Z`
- `1 year ago` means `2025-04-07T16:42:00Z`

In particular, if the reference date is `2026-01-31T16:42:00Z`, then `in 1 month` means `2026-02-28T16:42:00Z` (not `2026-02-31T16:42:00Z`, which is invalid; not `2026-03-03T16:42:00Z`, which is not exactly 1 calendar month later).

## Formatting Specs

Formatting prefers the most concise phrase that preserves semantics.

The decision order is:

1. Try the relative date spec clauses in the order of day words and then weekday words. If any of them matches, use the corresponding relative date spec clause. If the time spec is the same as the reference time, omit the time spec clause.

2. Try the calendar offset forms. If any of them matches, use the corresponding calendar offset form.

3. Try the absolute date spec clause. If the time spec is the same as the reference time, omit the time spec clause. This form is always available.

## Parsing Specs

The parser is not case-sensitive for ASCII letters.
