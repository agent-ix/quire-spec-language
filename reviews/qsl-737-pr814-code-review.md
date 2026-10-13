---
id: SR-9018
title: "Code review of quire-spec-language PR #814: run-level dispatch vectors for FR-359-AC-8"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@c7e80a9d1a36127e4a1107b02594b4d9b0369d11; qsl-eval/tests/it/dispatch_calls.rs (the only file the PR changes, +236/-0); gate logs qsl737u-fmt, qsl737u-clippy, qsl737u-gate, qsl737u-upcast-red, qsl737u-upcast-green"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-359
    type: reviews
---
# Code review of quire-spec-language PR #814

## Summary

Ticket: QSL-737. Base `impl/business-model-meanings`, head
`feat/upcast-run-level-vectors`. One commit, c7e80a9d1. The diff is
test-only: it changes only `qsl-eval/tests/it/dispatch_calls.rs` (+236, -0).
It has no production, lock, or spec change. The Rust lane (`rust-review`) is
folded into this file.

The PR adds the helpers `money_family`, `money_bridge`, `run_money_call` and
`completed_integer`, and two tests tagged `#[trace("FR-359-AC-8", "TC-1001")]`:

- `a_redeclared_subtype_body_runs_with_a_child_argument` (rooted at `B`)
- `the_ancestor_body_runs_with_a_child_argument_through_the_original_root`
  (rooted at `A`)

## Verdict

**CONDITIONAL.** The test code is idiomatic. It has typed assertions only
(`Integer`, `UndefinedReason::PreconditionFalse`, the `selected` record
field, and `IllTypedCause::TypeMismatch` with its catalog code). It has no
`#[ignore]`, no message-text assertion, no clock and no ambient state. Each
assertion fails for the change it guards:

- Running `A`'s body for a `B`-rooted call gives 5, not 105.
- Running `B`'s body for an `A`-rooted call gives 109, not 9.
- Refusing the child argument makes the `check_clause_expression` unwrap panic.
- Admitting a `Fee` makes the `expect_err` call fail.
- Dropping or conjoining the guard changes the amount-4 and amount-5 results.

The red run (`qsl737u-upcast-red.log`, 104 at amount 4) was correct
behaviour under FR-151, not a code defect. With `A.size` given no
precondition, `B`'s effective precondition is `B`'s own clause OR `true`,
which is `true`. The final fixture gives `A` an inert `>= 1000` guard, and
this matches FR-151's rule (an absent precondition is `true`, and the
effective precondition is a disjunction). It was not reverse-engineered to
pass.

Fmt (EXIT 0), workspace all-features clippy (run in the PR worktree, EXIT 0)
and the focused run (`qsl737u-upcast-green.log`: dispatch_calls, 35 passed,
the two new tests included) cover the head.

The sem+eval+replay gate log does not cover the head (FND-001). FND-002 and
FND-003 are documentation and oracle nits.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The sem+eval+replay+xtask gate log is not evidence for the PR head. `qsl737u-gate.log` compiled every qsl crate from `/home/peter/dev/quire-spec-language`, the main checkout on `main`. It is not the PR worktree. It resolved quire-exact 1d178c9c and QSV 010dca02 from main's lock; the head's lock pins 4c27c08 and ce9a0f1. None of the dispatch_calls child-argument tests appear in it, whether new or base. Its "0 failed, 1 ignored" therefore describes `main`, and the full gate has not been run on c7e80a9d1 | /home/peter/dev/worktrees/logs/qsl737u-gate.log:4-18,1983 |
| FND-002 | low | The doc of `the_ancestor_body_runs_with_a_child_argument_through_the_original_root` calls the `A`-rooted call "the upcast of `B`'s redeclaration", and the PR title says "operation upcast". In this checker and in FR-151 ("reference upcasts only"), the Upcast is the typing of a dispatch argument at its parameter. That step runs `Frame::Upcast`, then `upcast`, then `Form::Argument`. Every dispatched call with a `TaxedMoney` argument goes through it, including both new tests and the earlier 14fda4097 test. A reader of the gap analysis would infer a distinct Upcast path that does not exist | qsl-eval/tests/it/dispatch_calls.rs:2956-2959; qsl-semantics/src/check/check/typing.rs:545; qsl-semantics/src/check/check.rs:2280-2294 |
| FND-003 | low | The claim "reads the child's `amount`" cannot be falsified by a positional field read. `amount` is field 0 of both `Money` and `TaxedMoney`, so a read that takes field 0 instead of reading by key passes every vector. This is the defect class of QSL-737 AC-1 | qsl-eval/tests/it/dispatch_calls.rs:2760-2761 |
