---
id: SR-1659001
title: "code-review and rust-review of PR #1659 (QSL-580k, quantified clock guards and resets by selection)"
type: SpecReview
analysis: code-review
date: 2026-10-26
scope: "agent-ix/quire-spec-language; PR #1659 git diff origin/qsl-580j-record-collections...369c03677: qsl-semantics/src/check/timed_model.rs, qsl-semantics/src/check/timed_model/quantified.rs, qsl-semantics/src/check/state_clause.rs, qsl-eval/src/simulation/model/{timed.rs,state_predicate.rs}, qsl-eval/src/value/expression/mod.rs, qsl-semantics/tests/it/clock_places.rs, qsl-eval/tests/it/timed_subject.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-230
    type: reviews
---
# SR-1659001: code-review and rust-review of PR #1659 (QSL-580k, quantified clock guards and resets by selection)

## Summary

Ticket: QSL-580. Head: 369c03677. I checked soundness first, with probes in the review worktree, and reverted the probes afterwards.

**The decomposition is exact.**
- **`forall`:** the positive form is the union over `s` in `[min, max]` of `size = s` and `B` at every slot below `s`. The negative form is the union over `j` of `size > j` and not-`B` at slot `j`.
- **`exists`:** it is the dual of `forall`.
- **Connectives inside `B`:** `not` swaps the two forms, `and`/`or` distribute, and `L implies R` is read as `not L or R`, with `L and not R` as its negation.
- **Probe:** `forall(e in self.ts: e.n = 2 implies e.x >= 1 ms)` over `[0, 3]` gives exactly 1 + 2 + 4 + 8 = 15 parts, with the element data negated per slot.
- **Equality and negation:** `not forall(e in self.ts: e.x = 2 ms)` gives 6 parts, `x < 2` or `x > 2` on each slot, because `negated()` of `=` is `[<, >]`. `not exists(e.x > 2)` gives one entry per size, each with `x <= 2` on the slots below it.

**ElementBinding typing is consistent.** A part's binders are typed in the order of `part.elements()`, after `self` and the operation's parameters. The eval side binds `over`, then the same `elements()`, in the same order. Any operation parameter trips `ParameterizedGuard` first (`parameters().len() > 1 + elements().len()`), so no positional mix-up is reachable. A reset predicate is typed with no operation and is bound as `self` then the element.

**Slots.** Constraints and resets are placed by slot index (`place_at`).
- A part whose element slot is not held is disabled. The `size` literal already implies that slot liveness.
- A reset by selection writes each held slot whose element satisfies `P`, read in the pre-state; every other slot keeps its value.
- Reordering and shrinking happen only through collection-valued writes (580l), so slot = element holds here.

**No clock read escapes a constraint.** Inside `B`:
- a clock-free subtree is a data literal;
- a clock-reading node is either a connective or a comparison of an element clock with a constant;
- anything else, including `self.x` inside the body, a nested quantifier that reads a clock, and arithmetic on a clock, is refused `clock-read-outside-constraint` or `clock-comparison`.

The probe of `forall(e in self.ts: self.x <= 3 ms)` was refused as expected.

**The eval side matches S3.** `guarded_steps` reads each part's data with the element values, and places the part's constraints on the part's slots.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | `element_clock` requires at least one field step below the binder (`steps.is_empty()` gives `None`), so the element of a bare clock collection (`b: Bag<Clock>`, body `e >= 1 ms`) can never be a constraint. It would be refused once such a field reaches S3. Today the IR intake refuses a non-required `Clock` member (`INVALID_CLOCK_FIELD`), so this is latent. | quantified.rs 291-313 |
| FND-002 | low | `ClockReset` and `ClockResets` drop their `Clone, Eq, PartialEq` derives, a public API narrowing that the PR does not mention. `DataLiteral::Element`'s `PartialEq` ignores the binding: correct only because each slot's literal is a distinct `Rc`. A comment there would prevent a later regression. | timed_model.rs 370-431, 1133-1142 |
| FND-003 | low | `element_type` takes the clause body's root span for every binder's refusal, not the quantifier's span. | timed_model.rs 2119-2140 |

## Verdict

Mergeable on code grounds; no code bug found. See SR-1659002 for the spec-coverage findings.

## Coverage

- Read the whole diff.
- Probes run and reverted with `git checkout --`:
  - `forall` with an implication;
  - a negated `exists`;
  - a mixed conjunction;
  - a body reading a clock of `self`;
  - an `exists` and a negated `forall` over equality;
  - an unbounded `Set`;
  - the eval-side reset with and without a shrinking data conjunct.
- Baseline tests pass: qsl-eval `--test it` 763 of 763, and qsl-semantics `--test it` 1306 (all but my own reverted probe).

## Dispositions

Round 1. The fix commit is a97e471b5. The range from 4a7649df9 also carries 580j's fix commits from below, which are out of scope here. Checked in a fresh worktree and target under locked-build at jobs 4:
- qsl-semantics `clock_places`: 12/12 passed.
- qsl-eval `timed_subject`: 22/22 passed.
- clippy `-p qsl-semantics -p qsl-eval --all-targets --all-features -D warnings`: exit 0.

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | `element_clock` no longer requires a field step, so the bare binder matches a `[Field, Slot]` place, and `e >= 1 ms` over a `Bag<Clock>` can be a constraint or a reset. It stays latent until the IR admits a bare clock collection (IR-1036). |
| FND-002 | fixed | `Clone` and `PartialEq` are derived again on `ClockReset`, `ClockResets`, `SlotSelection` and `CheckedTimedPredicate`. `Eq` is not derived, which is correct because `CheckedBody` has no `Eq`. Two `DataLiteral::Element` values are now equal exactly when they have the same binding and the same renamed expression: semantically the same predicate over the same element, so equality and contradiction detection in the DNF stay sound. |
| FND-003 | fixed | Each element binding carries its quantifier's span, and `element_type` refuses there. No new test pins the span; low. |
