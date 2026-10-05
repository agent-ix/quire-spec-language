---
id: SR-1319
title: "QSL-482 code review of PR #633 (S2 declaration token lookup by binary search, 200k cancel test, parse-volume probe)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@1b7e37d85f032bcdeabbdae6fecf535a2d9f66d9; PR #633 diff against origin/main; qsl-forms/src/dispatch.rs; qsl-forms/src/value.rs (significant_tokens, context); qsl-replay/src/spine/lifecycle/tests.rs; qsl-bench/src/parse.rs; qsl-bench/src/bin/qsl-bench-probe.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-276
    type: reviews
---
## Summary

Ticket: QSL-482. PR: quire-spec-language#633. Code review with the rust-review
lane folded in. No build was run; `make ci` exit 0 on this head is recorded in
the caller's log, written after the head commit.

- **Binary-search slice.** `value::significant_tokens` (already on main, used by
  `value.rs` and `protocol_clause.rs`) takes `partition_point(start < span.start)`
  then `take_while(end <= span.end)` and filters `TokenClass::Token`.
  `LosslessCst::tokens` is documented as lossless leaves in source order, so
  token starts are monotone and tokens do not overlap. The old predicate
  (`start >= span.start && end <= span.end`) selects the same contiguous run:
  a token straddling either span edge is excluded by both, a zero-width token at
  `span.end` is included by both, and an empty or comment-only declaration gives
  an empty vector in both, so `leading_token_spelling` still returns `None` and
  `declared_extent` still finds no `bound`. Behaviour is unchanged; cost per
  declaration drops from O(total tokens) to O(log n + declaration tokens).
- **Cancel test.** `DECLARATIONS = 200_000` matches FR-276-AC-2. The test runs and
  passes in the gate log (it runs over 60 s in the test profile, which is the
  cost of a full uncancelled count plus the cancelled run). Its trace
  `TC-757, FR-276-AC-2` is unchanged and correct. It also serves as the de facto
  regression guard against the quadratic step returning, since a quadratic S2 at
  200k would not finish.
- **Probe.** `parse_with` and `parse-volume <N>` are small and correct;
  `s2_built=false` when S1 refuses.

## Verdict

Approve. The fix is correct at declaration boundaries and for empty or
comment-only token runs; no panics, no unchecked indexing (`get(first..)` with
`unwrap_or_default`).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |
