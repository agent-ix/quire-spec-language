---
id: SR-1544002
title: "gap analysis of QSL-1544 checked_views against FR-515 and TC-1310 (quire-spec-language PR 2156)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@51f6c0934b0bd7a187652e693e804a2d62432f05; spec/functional/FR-515-read-the-checked-predicate-and-temporal-subject-from-a-checked-package.md, spec/test-cases/TC-1310-read-the-checked-predicate-and-temporal-subject-from-a-checked-package.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-515
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-1310
    type: reviews
---
# SR-1544002: gap analysis of QSL-1544 checked_views (PR 2156)

## Summary

Ticket: QSL-1544. Plan completion: not assessed (no plan supplied). `quire matrix --scope .` at
the PR head lists FR-515-AC-1..7, 10..12, 15..17 as tagged, AC-13 and AC-14 as
tagged-by-ignored-test, and AC-8 and AC-9 as untagged. AC-to-code trace: AC-1 (recipe test in
tests/it plus the per-profile unit test), AC-2/10 (`the_activation_and_its_guard...`), AC-3,
AC-4/17, AC-5, AC-6, AC-7, AC-11, AC-12, AC-15, AC-16 each have a value-asserting test; AC-8 and
AC-9 are doctests. No stubs: the only `unimplemented!` are two test helpers reached only by the
ignored AC-14 test. No code without an owning requirement. Gated AC-13 and AC-14 stay as ignored
cases with real bodies citing IR-1279 and IR-1278, which is what the planner ruled; they are
counted as gated, not as passing coverage.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | FR-515-AC-8 and FR-515-AC-9 are untagged in the computed matrix. The compile-fail proofs are doctests, which cannot carry `#[trace]`, and TC-1310 says to tag the test for each criterion it covers. The repo's precedent (`qsl-semantics/tests/it/moved_item_paths.rs`) is a tagged `#[test]` that asserts the `compile_fail,<code>` doctest blocks exist for each path. Add one for the AC-8 blocks (constructor, five method names) and the AC-9 paths so the matrix binds them and a deleted doctest turns a test red. | qsl-package/src/checked_views/subject.rs:63-122; src/protocol_artifact/mod.rs:10-25; tests/it/checked_views.rs | correct-requirement-no-evidence |
| FND-002 | low | TC-1310 row 1 and AC-4 name the recipe packages R and the `view` binder, but `tests/it/checked_views.rs` calls `checked_predicate` on an R package only inside the ignored AC-14 test. The `view`-only binders, HoldsLeaf kind and formula-root span are asserted on synthetic `c`/`n` clauses only. Add a `checked_predicate` read of the recipe `Due` leaf asserting `binders() == [view]`. | tests/it/checked_views.rs:94-180 | correct-requirement-no-evidence |

## Verdict

Every AC has an owning test or a named gate. One medium (AC-8/AC-9 not bound in the matrix) and
one low (recipe packages exercise no predicate view). The two gated ACs are correctly gated.
