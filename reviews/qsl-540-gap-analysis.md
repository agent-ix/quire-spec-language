---
id: SR-12206
title: "Gap analysis of quire-spec-language PR #1054: E12 symmetry reduction and state constraints on safety"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@0c9ee6b172f48117ae13d7fdfaf30af911433463; FR-151-AC-1..4, FR-152-AC-1..4, FR-158-AC-1, FR-158-AC-3, FR-159-AC-1, FR-159-AC-2 (claimed whole) and the declared partial coverage of FR-158-AC-2, FR-158-AC-4, FR-159-AC-3, FR-160-AC-1..3, FR-160-AC-5, FR-161, FR-162-AC-1, FR-162-AC-4, against qsl-analyze/tests/it/reduction.rs, qsl-analyze/tests/it/reduction_world.rs, qsl-eval/src/simulation/reduction.rs (unit tests)"
review_set: subset
---
# Gap analysis of quire-spec-language PR #1054

## Summary

Ticket: QSL-540. A planless gap analysis (plan completion was not assessed),
scoped to the PR diff. Every `#[trace]` the PR adds was read against its AC to
confirm the test can fail on the AC's behaviour. Expected values were checked
for independence from the engine: the counts 27, 10, 18, 20, 12 and 9, and the
paths. They come from the spec's ADR-021 worked examples. The atoms are test
closures over `x.v`, not the engine's evaluator.

Examined, claimed whole:

- FR-151-AC-1 (TC-567): the admitted group, the generators `(a b)` and
  `(a b c)`, and one instance bound to `a` with orbit `[a, b, c]` and
  stabiliser `[b, c]`. Clean.
- FR-151-AC-2 (TC-567): `NotSymmetric`, `UnknownKey{d}`, `RepeatedKey{b}`
  and `NoUniverse`, each a refusal before exploration, with code and cause.
  Clean.
- FR-151-AC-3 (TC-568): `SymmetryBroken{counters, InitialStatesNotClosed{0,
  (a b)}}` with zero stats, then `[[a], [b, c]]` giving instances `a` under
  `[b, c]` and `b` with orbit `[b, c]` under the trivial group. Clean.
- FR-151-AC-4 (TC-568): the open set settles `SymmetryBroken` at initial 1,
  and the closed set admits. Clean.
- FR-152-AC-1 (TC-569): the three canonical forms and their permutations.
  Clean.
- FR-152-AC-2 (TC-569): 27 reachable states give 10 and 18 forms. For each
  state, `π(s)` equals the form and the form is idempotent. The runs store 10
  and 18 states. Clean.
- FR-152-AC-3 (TC-569): one orbit for the two linked states. Each state's
  form lies in the orbit and equals `π(s)`. Every orbit of the 24 states has 1
  to 2 forms. The forms are measured on `canonicalise` over every orbit
  member, which bounds what a run stores. Clean.
- FR-152-AC-4 (TC-569): repeated calls agree, and the trivial group gives
  the identity. Clean.
- FR-158-AC-1 (TC-580): constrained and unconstrained prefixes are equal,
  `bump(a), bump(a)` to `(2,0,0)`. The stored and boundary counts are 20 and
  12, so 8 states are expanded. Expanding a boundary state would change the
  count. Clean.
- FR-158-AC-3 (TC-581): see FND-001.
- FR-159-AC-1 (TC-582): TP-2 and TP-3 with an interval settle
  `NotPreserving{PartialOrder, …}` with zero stats. Under symmetry they are
  admitted. Clean.
- FR-159-AC-2 (TC-582): every partial-order row, the symmetry and constraint
  rows, and the TP-2 symmetry-plus-partial-order request naming partial-order
  reduction. Clean.

The partial coverage is declared. The untagged tests say which clauses they
cover: FR-158-AC-4 determinism, FR-159-AC-3's first sentence, FR-162-AC-1's
hooks and simulation, FR-162-AC-4's limit clause, FR-160-AC-1, AC-2, AC-3 and
AC-5 (safety rows, AC-5 without its replay), and FR-161's start and step rules
with an FR-128 replay of a deadlock path. The uncovered ACs belong to the
ticketed 9-item remainder (liveness, partial-order reduction, identity and
negotiation, per-orbit records, `each` loops) and are not findings.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Over-tag: `undefined_absent_and_initially_false_constraints_and_bounds` is traced to the whole of FR-158-AC-3, but the AC's first clause requires "a replaying counterexample ending at that state". The test re-executes the prefix through `ModelSystem::successors` (`World::positions`) and never calls FR-128's `replay_model_trace`. That replay could not reproduce the counterexample anyway, because it has no `UndefinedLocus::Constraint` handling (SR-12205 FND-002). The test cannot fail on the replay behaviour the AC states. Fix: implement constraint-locus replay and assert `Reproduced` here, or drop the `#[trace]` and declare the AC partial, as the other partial tests do. | qsl-analyze/tests/it/reduction.rs:627-690 |

## Verdict

Changes requested on one finding. Eleven of the twelve whole-AC claims are
sound: each tagged test asserts the AC's own values and would fail if the
behaviour broke. FND-001 over-claims FR-158-AC-3's replay clause. Fix it by
adding the replay or by removing the tag. The overlapping-declaration
soundness defect (SR-12205 FND-001) has no test. Its regression test belongs
in that fix, traced to FR-151-AC-2 only if the AC is widened to name it.

## Dispositions

Round 1, reviewed at `0f70ab51e61f8e77809be97ec3f8fdffe674d91a` (fix commit
`1263568b05c07035cdde6bf7a5c73d06c185189b`).

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 1263568b05c07035cdde6bf7a5c73d06c185189b |

FND-001: the FR-158-AC-3 test now runs the deadlock-freedom item under the
undefined constraint `Odd`. It asserts the `Constraint("Odd")` locus and
replays the counterexample through FR-128's `replay_model_trace` (the
`World::replay` helper) to `Reproduced` with the claimed undefined
evaluation. The test can fail on the AC's replay clause. Authored-clause
replay still refuses until FR-337. That is a pre-existing limit, outside
this AC's constraint behaviour, and it is recorded in QSL-913 item 8.
