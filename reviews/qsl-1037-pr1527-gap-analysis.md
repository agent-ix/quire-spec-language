---
id: SR-17205
title: "QSL-1037 gap analysis of FR-337-AC-6 and FR-337-AC-7 (quire-spec-language PR 1527)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@a35be7dc256a30941b6be8ef5256a445f41950c4; spec/functional/FR-337-*.md; spec/test-cases/TC-1265-*.md; spec/functional/FR-125-*.md (AC-18, context); qsl-package/src/emit/tests/temporal_clause.rs; qsl-semantics/src/check/temporal_clause/tests.rs; qsl-semantics/src/check/{temporal_clause.rs,temporal_clause/condition.rs,lowering/temporal.rs,mod.rs}"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-337
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-1265
    type: references
---

## Summary

Ticket: QSL-1037 (PR quire-spec-language#1527, PR 1 of 2). This is a
planless gap analysis with semantic review done inline. The criteria in scope
are FR-337-AC-6 and FR-337-AC-7, and the TC-1265 procedure. FR-125-AC-18
(production letter seam) is PR 2's and was read for context only: the PR
does not claim it and no test tags it. The same holds for FR-126-AC-16 and
TC-1201.

Computed matrix (`quire matrix --scope .`, quire 0.36.2, at HEAD): FR-337-AC-6
and FR-337-AC-7 are tagged; FR-125-AC-18 is untagged (PR 2's criterion). The
tagging tests are:

- FR-337-AC-6:
  `emit::tests::temporal_clause::an_on_each_clause_emits_its_trigger_and_guard`
  (TC-1265).
- FR-337-AC-7:
  - `emit::tests::temporal_clause::s3_checks_the_on_each_guard_and_binder`
    (TC-1265).
  - `check::temporal_clause::tests::an_on_each_guard_is_kept_with_the_emitted_clause`
    (TC-1265).
  - `check::temporal_clause::tests::the_package_check_refuses_a_clause_no_emission_writes`
    (FR-326).

Every binding is correct. Each test asserts what its tag claims.

AC-6, clause by clause: all covered.

- `kind` `each`.
- `trigger` is the level-1 `parameter` node `q`, typed as `over` is.
- `guard` reaches `c`'s level-0 node and not `q`.
- The guard-less clause has no `guard`. `when (true)` has one. `on origin`
  has one member.
- Four distinct keys and `package_id`s.
- The changed guard and the renamed binder change the key. Reformatting
  changes neither key nor id.
- The occurrences are at `q`.
- I2 admits each package and returns the triple and `over`.

AC-7, clause by clause: all covered except "naming both declarations"
(FND-001). Each covered case asserts its code, tag and region start:

- `q.value = 1` checks and reaches the trigger node.
- `non-boolean-root`, `missing-name` at the body's `q`, `unproved-range` at
  `c.value + 1`.
- The `mod` guard checks.
- `missing-name` at `Absent`, `type-mismatch` at `Integer`,
  `duplicate-declaration` at the binder `c`.
- One refusal, at the guard.
- A capture is `NotEmitted` at the clause.

Gates run on a private detached worktree at HEAD. Target dir:
`~/.cache/qsl-review-1527`, serialized with `flock /tmp/qsl-cargo.lock`.

- `cargo test -p qsl-package -p qsl-semantics --lib temporal_clause`: 10 and
  14 passed.

Mutations against that command:

| Mutation | Result | Caught by |
|---|---|---|
| M1: drop the `T != U` check | killed | s3_checks |
| M2: `first` = binder span | **survived** | |
| M4: drop the guard from `named_populations` | **survived** | |
| M5: drop guard objects from the requirement record | **survived** | |
| M5b: drop guard named domains | **survived** | |
| M6: trigger level 2 | killed | emits |
| M7: drop the trigger `type` occurrence | killed | emits |
| M9: refuse `on each` again | killed | |
| M10: guard typed without `q` | killed | |
| M11: drop the duplicate-name check | killed | |
| M12: guard not lowered | killed | |
| M13: guard typed after the `holds` conditions | killed | |

Reviewer probes P1 and P2 (production letters, trace spine) are in SR-17204
FND-001.

## Verdict

PASS-WITH-GAPS for the two in-scope ACs. FND-001 is an AC-7 clause with no
assertion. FND-002 is behaviour FR-337 states ("the populations it names join
the clause's named populations") that no test pins, and the requirement
record's domains are wire-visible. FND-003 is low. Merge readiness is decided
by SR-17204 FND-001.

## Findings

| ID | Severity | Summary | Refs |
|----|----------|---------|------|
| FND-001 | medium | FR-337-AC-7's "`on each (c: M::Counter)` refuses `conflicting_declaration`/`duplicate-declaration` at the binder `c`, naming both declarations" is tested only for code, tag and region start. `DuplicateTrigger.first` (the `over` declaration) is never asserted, and mutation M2 (`first` = the binder's own span) survives | qsl-package/src/emit/tests/temporal_clause.rs:994-1001; qsl-semantics/src/check/temporal_clause.rs:898-904 |
| FND-002 | medium | FR-337 "`on each` activation at S3" says the guard is a population position whose named populations join the clause's, and the PR adds the guard to `named_populations`, the requirement record's objects and its unbounded population domains. No test has a guard that names a population, and mutations M4, M5 and M5b (dropping each of the three) all survive | qsl-semantics/src/check/mod.rs:697-705,3730-3746 |
| FND-003 | low | Two AC-6/S3 details are untested. `when (true)` is asserted only to bind some `guard`, not "the lowered term of `true`". The guard's nodes being "located at the guard's own expression tree" is checked only through refusal regions, not through emitted guard-node occurrences | qsl-package/src/emit/tests/temporal_clause.rs:872,878 |

## New findings (disposition pass 1)

Disposition pass 1 reviews head `bbc203e57`, with PR 2 folded in.

Mutations were re-run against `cargo test -p qsl-package -p qsl-semantics
--lib` plus `cargo test -p qsl-analyze --test it model_instances`:

- M2 killed by `a_repeated_binder_name_refuses_naming_both_declarations`.
- M4 killed by `the_populations_a_guard_names_join_the_clauses`.
- M5b killed by `the_populations_a_guard_names_join_the_clauses`.
- M5 (guard objects dropped from the requirement record's
  `ModelReferences`) **survived**.

The coder's reason for leaving M5 untested is that
`RequirementRecord::references()` is `pub(crate)`. That does not stop a
test: the objects are observable through the public
`CheckedGraph::item_elements` (ADR-017 AR-4 referenced elements), and an
in-crate test in `qsl-semantics` can also read `references().objects`
directly.

| ID | Severity | Summary | Refs |
|----|----------|---------|------|
| FND-004 | medium | FR-125-AC-18 is now claimed (three tests tagged), but most of its rows are not exercised through the production letter seam. Uncovered: the Counter event-position rows (`when (c.value = 1)` Violated with prefix 0→1→2, `when (c.value >= 2)` Holds{Exhaustive}); the infinite-trace `when (c.value >= 3)` row with its prefix and the FR-128 replay to `reproduced-with-evaluated-witness` (`ReplayedLetters::activation` is untested, as model_check_safety.rs's module doc says); the `6 mod (2 - c.value)` guard's UndefinedEvaluation at position 2 naming the activation; and the POR visibility of `inc` under the `c.value >= 3` footprint. In addition, `the_production_letters_read_the_counter_guard_and_its_footprint` asserts only `Violated` for `when (q.value >= 1)` + `always holds(c.value <= 2)`, which an activation-everywhere reading also gives | qsl-analyze/tests/it/model_check_safety.rs:464-530; qsl-analyze/tests/it/model_instances.rs:1022-1084 |
| FND-005 | low | `the_populations_a_guard_names_join_the_clauses` is tagged `FR-125-AC-18`, but it checks FR-337 "`on each` activation at S3" (the guard is a population position, FR-391), not the letter seam that AC-18 states | qsl-analyze/tests/it/model_instances.rs:1090-1092 |

## New findings (disposition pass 2)

Disposition pass 2 reviews #1538 at `3637c7461`. Mutation M5 was re-run
(guard objects dropped from the record's `ModelReferences`; `cargo test -p
qsl-package -p qsl-semantics --lib` plus `-p qsl-analyze --test it
model_instances`). It is now killed by
`the_populations_a_guard_names_join_the_clauses`, which counts
`AbstractionKey::Object` elements through the public
`CheckedGraph::item_elements`.

The FR-125-AC-18 tag now sits on
`model_check_safety::the_production_letters_read_the_counter_guard_and_its_footprint`
alone. The two model_instances tests are retagged `FR-337`. The rows that
are still uncovered, including that test's non-discriminating `Violated`
assertion, are filed as QSL-1086.

No new findings.

## Dispositions

| FND | outcome | reason |
|-----|---------|--------|
| FND-001 | fixed | `a_repeated_binder_name_refuses_naming_both_declarations` asserts `DuplicateTrigger { first }` at the `over` name. M2 is killed |
| FND-002 | still-open | M4 and M5b are now killed by `the_populations_a_guard_names_join_the_clauses`. M5 still survives: no test shows the guard's objects joining the requirement record's references. The stated blocker (`references()` is `pub(crate)`) does not hold, because the public `CheckedGraph::item_elements` exposes them and an in-crate qsl-semantics test can read them. The fix is a small in-PR test; it could also be deferred to a ticket by the lead |
| FND-003 | fixed | `when (true)` is asserted to lower to the Boolean literal `true` located at `true`, and the guard's emitted node occurrence is located at the guard's text |
| FND-002 | fixed | `the_populations_a_guard_names_join_the_clauses` asserts that a guard reading `y.age` adds one `AbstractionKey::Object` through `CheckedGraph::item_elements` (none for `when (true)`). M5 is killed |
| FND-004 | deferred | QSL-1086 (Backlog) owns the FR-125-AC-18 rows not run through the production letters and replay. The AC-18 tag is narrowed to the guard-reading/footprint test |
| FND-005 | fixed | Both model_instances tests are now tagged `#[trace("TC-520", "FR-337")]` |
