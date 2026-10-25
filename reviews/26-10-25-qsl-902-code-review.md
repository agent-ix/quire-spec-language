---
id: SR-17204
title: "Code review of QSL-902 observed temporal replay"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language PR #1531; git diff 9c7a006bd..qsl-902-code; qsl-replay and qsl-eval observed temporal replay path, result, wire envelope, and downstream envelope fixtures"
review_set: subset
date: '2026-10-25'
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-331
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-071
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-072
    type: reviews
---
## Summary

Ticket QSL-902. The code adds an observed temporal replay arm, reconstructs the
wire trace, runs the already checked clause through the temporal evaluator,
and returns the decoded trace position with the typed evaluation and parity
settlement. Common request decoding, dependency validation, compilation and
package identity checks run before dispatch. The model-trace replay API remains
unchanged. The observed source uses a non-recursive envelope source type.

The new tests exercise lasso and bounded formula parity, undefined-value
reproduction and mismatch, fairness refusal, wire round-trip and object
resolution, wrong source/target, clause and envelope mismatch, missing or
invalid trace forms, and incomplete evaluation. `cargo test -p qsl-replay -p
qsl-eval` passed (866 qsl-replay unit tests, all integration tests, 103
qsl-eval integration tests, and both crates' doctests). `cargo fmt --all --
--check`, `git diff --check`, and `cargo clippy -p qsl-replay -p qsl-eval
--all-targets --all-features -- -D warnings` passed.

One acceptance-evidence gap remains: FR-331-AC-8's `InsufficientData` case is
currently asserted by directly settling an `ObservedTemporalResult`, not by
replaying a request through `qsl_replay::replay`. The same end-to-end test
does cover `Incomplete`; the facade preserves the evaluator's typed outcome
in its generic result path. The documented aggregate source fixture was not
usable for an end-to-end replay in this checkout: the parser rejects its
`count where` term at `where` (expected `(` or `<`).

## Verdict

Conditional, with one medium evidence gap. The result settlement and facade
behavior align with FR-331; no behavior or criterion was removed. The
`InsufficientData` integration path should gain an end-to-end fixture when a
valid checked temporal aggregate source is available.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-331-AC-8 requires replay of an `InsufficientData` outcome through the facade and retention of its complete fields. The test constructs that outcome and calls `ObservedTemporalResult::settle` directly; it does not exercise `replay`, trace reconstruction, or the evaluator result plumbing for this variant. The attempt to use TC-847's documented aggregate source failed at parse time (`count where`, error at `where`, expected `(` or `<`). Add an end-to-end replay fixture once the source can compile, asserting position, atom locus, count, min_count and `ReplayParity`. | qsl-replay/src/spine/clause/tests/temporal.rs:343; spec/test-cases/TC-847-an-undefined-letter-fails-a-supplied-trace.md:60-62 |

## Dispositions

None.
