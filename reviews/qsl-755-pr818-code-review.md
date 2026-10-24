---
id: SR-9049
title: "QSL-755 quire-spec-language PR 818 code review with rust-review lane"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@7665c03365a107fc52bd7b0e741e61a4fee68a24; qsl-semantics/src/model/intake.rs; qsl-semantics/tests/it/tc235_state_machine.rs"
review_set: subset
---

## Summary

Ticket: QSL-755. Base `impl/business-model-meanings`, one commit. The diff
touches two files: `duplicate_process_step_refusal` in
`qsl-semantics/src/model/intake.rs` and one new test in
`qsl-semantics/tests/it/tc235_state_machine.rs`.

The change adds a `"clauses"` arm to the member match and makes the owning
meaning optional, so the meaning check runs only for steps, states and
transitions. The three existing arms keep their meaning check
(`Some(meaning::...)`), their key choice (a step names the process, a state
or transition names its own identity) and their member path, so their
behaviour is unchanged.

Checked against the pinned semantic-IR reader (`rules.rs`
`identity_declarations` at the Cargo.lock rev): the walk order is type,
constraints, fields, variants, relationships, operations with their params,
then clauses, states, transitions, steps. The later declaration is flagged.
A clause therefore always reports at `/clauses/j/identity` when it collides
with an operation or field of its owner, whichever the author wrote first.
A collision between two clauses also reports there. QSL maps all of these to
`conflicting-binding` (QSpec TC-235 / FR-154-AC-12, cited by id). The wider
arm is correct.

Rust lane: no new `unwrap`, `expect`, `panic` or indexing in src. The
function still returns `Option` and short-circuits with `?`. There are no
integer conversions, no new allocations on a hot path, and no ticket ids in
code. The test's `unwrap` and `panic!` in let-else follow the file's
convention. The coder's gate log shows clippy `-D warnings` and fmt pass.
Reviewer-only: nothing was built here.

## Verdict

PASS WITH FINDINGS (one low). The mapping is correct and minimal, and the
state, transition and step arms are unchanged. The only finding is a doc
comment that no longer describes what the function does.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The rustdoc of `duplicate_process_step_refusal` still says "FR-208 process binding owns duplicate step identities", but the function now maps step, state, transition and clause duplicates, and a clause needs no owning meaning. A reader of the doc would not expect it to handle a clause on a record or an event. | qsl-semantics/src/model/intake.rs:1643-1646 |
