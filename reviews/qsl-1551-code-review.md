---
id: SR-2159001
title: "Code and Rust review of the QSL-1551 main-red fix (quire-spec-language PR 2159)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; qsl-semantics/src/check/timed_model/quantified.rs; qsl-semantics/tests/it/model_dispatch.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-230
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-083
    type: references
---

# Code and Rust review of the QSL-1551 main-red fix (PR 2159)

## Summary

Ticket: QSL-1551. Reviewed `git diff origin/main...HEAD` at head d340d0b5f (2 files). Both root causes
verified in code and history. (1) PR 2126 (b40dda28a) builds the bounded quantifier size expression with
`expression_spans_at`, which needs one source position per generated node; a quantifier read through a
clock-reading declaration is located at its call as an empty point span and the division returns
`SpanRefusal::Reversed`, mapped to `ClockReadOutsideConstraint`. (2) PR 2047 (534320462) made the checked
dispatch bridge read the root member name via `operation_anchor_declaration`, which calls
`member_identity_name(owner, key)` and so needs `<owner>/<member>` identities; the chain fixtures used
`model.A.op<N>`.

Mutation evidence: reverting only the quantified.rs change makes
`a_quantified_read_expands_in_a_clock_reading_declaration` fail with `ClockReadOutsideConstraint { span 419..419 }`;
reverting only the fixture changes makes both bridge tests fail with `MissingClauseData { field: Member }`.
Both fail on the old code and pass on the new. The clock test asserts concrete guard entries per size 0 to 3
and per slot, so it is value-asserting. The fixture change renames identity strings only: EDGES (130 and
the walk-count constants), the `family_steps` bounds and every assertion are untouched; the >128-step and
walk-counting tests still run those walks.

Other callers of `expression_spans_at`: quantified.rs:203 (size expression, range
`min(call start, body start)..body start`) and :560 (body substitution at `located.span(id)`); both are
point-safe through the early return. `ExpressionSpans::at_point` is the existing helper already used by
timed_model.rs:4095 and reading.rs:1068 for grafted nodes, so the point placement matches the convention
for generated nodes with no source range; no diagnostic span changes for written (non-empty) quantifiers.

Gates run on the PR head: the three tests and the full `qsl-semantics --test it --all-features`
(1314 passed, 0 failed); `cargo check` and `cargo clippy -D warnings` for the workspace with default
features and with `--no-default-features` (clean); `make arch-lint-qualified-core` (direction and ambient
input PASS). The diff adds no `#[allow]`, `unwrap` or `expect`, and no pin. The head is one commit behind
origin/main (e9a388154); `git merge-tree` shows a clean merge.

## Verdict

**PASS (mergeable)** with two low findings (stale comments, one unpinned placement).

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | low | The doc comment of `expression_spans_at` still says every generated node has a "non-empty provenance region", but the new early return gives every node an empty point span. State the point case in the doc comment (the inline comment explains it, the contract does not). | qsl-semantics/src/check/timed_model/quantified.rs:323-327 | correct-requirement-no-evidence |
| FND-002 | low | The `redefinition_chain` doc comment now reads `model.A.op0 <- model.A.op1 <- ... <- model.A/op{edges}`, mixing the old and new identity forms in one chain; the chain is all `<owner>/<member>`. | qsl-semantics/tests/it/model_dispatch.rs:570-571 | correct-requirement-no-evidence |
| FND-003 | low | No test pins where the generated size-expression nodes sit for the point case (all at the call point, empty spans); TC-685 asserts only the recorded guards, so a regression that places them elsewhere or drops provenance on the point path passes. Optional: one assertion on the form's spans. | qsl-semantics/tests/it/clock_places.rs:1380-1440 | correct-requirement-no-evidence |
