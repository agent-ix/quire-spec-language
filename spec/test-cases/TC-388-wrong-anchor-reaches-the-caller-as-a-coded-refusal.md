---
id: TC-388
title: "An evaluation-time wrong-anchor snapshot reaches the caller as a coded QSL refusal, not a kernel refusal"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-090
    type: verifies
---
# TC-388: An evaluation-time wrong-anchor snapshot reaches the caller as a coded QSL refusal, not a kernel refusal

## Description

Verify FR-090-AC-7. Suppose a postcondition's `pre(..)` read is evaluated
over a population argument that was admitted with no pre anchor. The caller
then receives an `Evaluation` whose `outcome` is
`FamilyOutcome::FamilyEvaluated(FamilyResult::Refused(cause))` and whose
`cause.catalog_code()` is `wrong_snapshot`/`wrong-anchor`. The result is not
a kernel `Outcome::Refused` inside `FamilyOutcome::Evaluated`, not an
admission refusal and not a panic. The kernel `Refusal` has no
`WrongSnapshot` variant (ADR-013 O-16, T-6). Scope: FR-090-AC-7.

This test replaces `tests/it/model_reference_queries.rs`'s
`pre_of_a_binding_with_no_pre_anchor_refuses_wrong_anchor` (FR-042-AC-4,
TC-198). That test asserts
`Outcome::Refused(Refusal::WrongSnapshot(WrongSnapshotCause::WrongAnchor))`
on QSL's kernel copy, which is the shape T-6 removes.

## Test Procedure

1. Reuse `tests/it/model_reference_queries.rs`'s scenario: a package with a
   postcondition parameter `p: Population<A>[3]` and the expression
   `pre(allInstances<A>(p))`.
2. Admit the population argument through `admit_binding` directly. This
   attaches no pre anchor.
3. Evaluate the checked postcondition through `CheckedPackage::evaluate`.
4. Match the result as `Ok(e)` and `e.outcome` as
   `FamilyOutcome::FamilyEvaluated(FamilyResult::Refused(cause))`, and take
   `cause.catalog_code()`.
5. Inspect `quire-exact/src/outcome.rs`'s `Refusal` enum.

Tag the test `#[trace("FR-090-AC-7", "TC-388")]`.

## Expected Results

- Step 4's code is `CatalogCode::new("wrong_snapshot", "wrong-anchor")`.
- Step 2 succeeds: `admit_binding` admits the argument, so the refusal is
  not an admission refusal.
- Step 3 does not panic, and its result matches step 4's pattern;
  `e.outcome` is not `FamilyOutcome::Evaluated(Outcome::Refused(_))`.
- Step 5 finds no variant whose payload is `WrongSnapshotCause`. This is
  also a compile-time fact: `quire-exact` cannot import `crate::check`.

## Status

Planned; no test backs this case.
