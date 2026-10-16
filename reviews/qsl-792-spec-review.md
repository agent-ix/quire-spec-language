---
id: SR-11750
title: "Spec review of quire-spec-language PR #907: classify, check and evaluate model type invariant clauses"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@e9f2557d914c01a5e42a3696601bfdf1430dfb43; spec/functional/FR-366-classify-check-and-evaluate-the-invariant-clauses-of-a-model-type.md, spec/functional/FR-106-admit-snapshots-and-invocations.md, spec/functional/FR-103-admit-model-operations-and-frames-on-the-spine.md, spec/spec.md, spec/test-cases/TC-1050-a-type-level-clause-is-classified-parsed-and-checked-as-an-invariant.md, spec/test-cases/TC-1051-invariants-of-record-and-event-types-are-evaluated-once-for-each-reachable-value.md"
review_set: subset
---
# Spec review of quire-spec-language PR #907

## Summary

Ticket: QSL-792. One pass of spec-review with the contradiction,
completeness, vector-soundness and API-fidelity sub-analyses over the PR
diff (`origin/main...HEAD`, 6 files). Spec-only PR, so no code review. Only
major issues block; mediums and lows are recorded in the Verdict.

Oracles: QSL on main: FR-056 (native equality, AC-35), FR-103, FR-104,
FR-106 (selection forms, checks 2-11), FR-107 (charges, read paths, clause
sets), `qsl-eval/src/value/expression/mod.rs` (`evaluate_clause`,
`ClauseSetup::of`), `qsl-eval/src/value/expression/evaluate.rs` (`Field`,
`Attribute`, `order` arms), quire-exact `numeric.rs` (`order` charges),
`qsl-semantics/src/model/observation.rs` (`ClauseSelectionInput`,
`AdmittedObservations`, `admit_observations`), `qsl-foundation/src/witness.rs`
(`RuntimeValuePath`), `qsl-semantics/tests/support/tc235_k2.rs` (K2),
`qsl-semantics/tests/it/tc235_record_event.rs` (M13, M14, M17),
`qsl-semantics/tests/it/tc235_state_machine.rs` (M06, M19),
`qsl-eval/tests/it/tc235_business_meaning_runtime.rs` (M16). QSpec by id:
FR-207, FR-208, TC-235 M13, M14.

Examined:
- FR-366 Description, Inputs, Outputs, Behavior (examined)
- FR-366-AC-1 to FR-366-AC-8 (examined)
- FR-106 `ValueInvariants` selection paragraph and FR-106-AC-48 (examined)
- FR-103 pointer sentence (examined)
- spec/spec.md relationship and index rows (examined)
- TC-1050 steps 1-8 and expected results (examined)
- TC-1051 steps 1-7 and expected results (examined)
- FR-106 checks 2-11 and Outputs; FR-107 charges, read paths, clause sets (context_only)

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | TC-1051 step 7 expects "the first two results and `Incomplete`" from a budget that denies the third charge. That holds only if each value costs one charge. FR-366 states no per-value charge, so the charges are FR-107's: the `self.amount_minor` record field read is uncharged (`Field` arm), and the integer `>=` charges `ordering.operands` then `ordering.arithmetic` (quire-exact `numeric.rs` `order`). Charge 3 is therefore denied inside the second value's evaluation. That leaves one completed result, then `Incomplete`. Fix: state the per-value charges, or state the budget as the charge log of the first two evaluations plus one. | spec/test-cases/TC-1051-invariants-of-record-and-event-types-are-evaluated-once-for-each-reachable-value.md:55,78; spec/functional/FR-366-classify-check-and-evaluate-the-invariant-clauses-of-a-model-type.md:177 |
| FND-002 | high | FR-366-AC-1 contradicts itself as written. In one fixture (K2 plus `Extra`) it says `Money/NonNegative` in `ocl` is `Unevaluated` and also that `TaxedMoney` has effective invariants `[NonNegative]`. By the classification rule, an `ocl` clause is in no effective-invariant list. In addition, K2 declares no `Money` clause, no `TaxedMoney` and no `ExpressLifecycle` (tc235_k2.rs). TC-1050 steps 2 and 3 correctly split the fixtures (ocl, then quire plus M17 `TaxedMoney`, and M19 run 1). Fix: name the same three fixtures in the AC. | spec/functional/FR-366-classify-check-and-evaluate-the-invariant-clauses-of-a-model-type.md:170 |
| FND-003 | high | Behavior says the type-level invariant of an object type, state machine or process is evaluated "over the `Current` or `Context` selection of FR-106 by FR-107's `evaluate_clause`". That path cannot reach such a clause. `evaluate_clause` and `ClauseSelection` resolve an unqualified unit state-clause name (`ClauseSetup::of` -> `graph().state_clause(name)`, otherwise `UnknownClause`), and FR-366 says a checked invariant "adds no declaration". No name, selection or API is stated, so this describes existing behaviour falsely. No AC or TC row evaluates such an invariant either (M06 run 4 `Extra` is classified only). Fix: state the new entry or the name under which a checked type invariant is selected, and add an AC and TC row (for example `Extra` over a `Context` selection). | spec/functional/FR-366-classify-check-and-evaluate-the-invariant-clauses-of-a-model-type.md:158-161 |
| FND-004 | high | No TC-1051 step covers part of FR-366-AC-7: "a value of an event type reached through a parameter is evaluated at the parameter binding", and "a `reference` field is not followed: the referenced object's values are reached once". Step 5 and expected result 5 exercise only `TaxedMoney` at `quoted` and `Pair`. No operation with an event-typed parameter and no `reference` field appear in the fixture, so neither clause can fail. Fix: add steps (an operation with an `OrderPlaced` parameter; a `Reference<Order>` field whose target holds a negative `Money`, expecting one result at the target's own member path) or drop the clauses. | spec/functional/FR-366-classify-check-and-evaluate-the-invariant-clauses-of-a-model-type.md:176; spec/test-cases/TC-1051-invariants-of-record-and-event-types-are-evaluated-once-for-each-reachable-value.md:49-52,72-75 |
| FND-005 | medium | FR-366-AC-4 and TC-1051 expected result 1 fix the parameter result `(pre observation of i1, parameter amount)` after the three `s1` results, "in this order". FR-207 key order compares the observation key first, and the fixture gives no labels for `s1` or `i1`'s `pre` snapshot. The position of the parameter result therefore depends on labels the row leaves open. QSpec TC-235 M13 orders only the three results under `s1`. A conforming implementation over other labels fails the row. Fix: give the two snapshots' labels so the order is determined, or assert key order only among the `s1` results. | spec/functional/FR-366-classify-check-and-evaluate-the-invariant-clauses-of-a-model-type.md:173; spec/test-cases/TC-1051-invariants-of-record-and-event-types-are-evaluated-once-for-each-reachable-value.md:61-66 |

## Verdict

FAIL, five blocking findings (FND-001 to FND-004 high, FND-005 medium; FND-005
blocks under the "TC row with a wrong or undetermined expected outcome" rule).

Traced and correct:
- Choice 1 (`evaluate_value_invariants` plus a `ValueInvariants` selection):
  sound as a stated new target. FR-106 already specifies the unbuilt
  `Context` variant the same way. Check 3 makes the `pre`-document refusal
  (`wrong-observation`) reachable, and check 10 makes the missing-parameter
  refusal (`missing-member`) reachable.
- Choice 2 (`ClauseRole` plus effective invariants): matches the merged M06
  run 4 test (`Extra` is the invariant, `NotYetPlaced` is the guard of
  transition `[0]`) and the M19 effective-machine test (inherited `[0]` guarded
  `NotYetPlaced`).
- Choice 3 (FR-207 key order, then clause key; a value at two locations is
  evaluated at each): consistent with the FR-207 path vocabulary in
  `witness.rs`. `RuntimeValuePath` implements no `Ord` today, so the order is
  new code, as the FR states.
- Choice 5 (M14 run 1 relies on native Timestamp equality): the rule exists
  and is merged, but it is FR-056-AC-35's, not FR-091's (see below).
- `forbidden-pre-read` and `non-boolean-root` exist in
  `check/refusal.rs`. `TypeDefinition.clauses[k]` locations match intake's
  existing convention. The M13 and M14 charge counts match the merged tests.
- FR-106-AC-48, FR-366, TC-1050 and TC-1051 are free on origin/main and in
  open PRs #905, #906 and #908. There is no roadmap language and there are no
  ticket ids in spec text.

Recorded, non-blocking:
- medium: FR-366 cites FR-091 twice for equality on native String, UUID and
  Timestamp (Behavior "as FR-091 states" and Dependencies). That rule is
  FR-056's ("Unit-language equality on native String, UUID and Timestamp",
  FR-056-AC-35).
- medium: The `ValueInvariants` admission reuses checks that are keyed on
  the selected clause, and none of them is defined for a selection with no
  clause: check 4 (the clause's alias), check 9 and the required-population
  rule ("the population holding `self`"), and `AdmittedObservations`'
  required `clause: NodeKey` and `self_object` fields. FR-106 Outputs do not
  describe the value this selection yields.
- medium: `evaluate_value_invariants` uses one shared `Meter`, while FR-107's
  clause set (whose combination it cites) builds a fresh meter for each
  clause from `ScalarLimits`. Say that the difference is deliberate.
- low: FR-366 cites FR-107's clause-set combination, which also has
  `Undecided`. Say why an invariant cannot reach it (Linked checking).

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-006 | high | The state-machine half of the new FR-366-AC-9 and TC-1051 step 11 cannot be reached over K2, the only base fixture TC-1051 names. The step evaluates `Extra` of `OrderLifecycle` over `i1` of `place`, with `pre` context `{placed: false, cancelled: false}` and `post` context `{placed: true, cancelled: true}`, and expects two `Completed(true)` results. K2's `OrderLifecycle` declares only the field `placed` (tc235_k2.rs), so FR-106 check 6.12 refuses `cancelled` as `unknown-member`. K2's `place` has an empty frame (`modifies: []`), so even without `cancelled`, check 11 (which FR-106 now applies to a `ValueInvariants` invocation "as an `Invocation` selection of a postcondition reads it") refuses the change to `placed` as `frame_violation`. The contexts match TC-1045's fixture: `cancelled` field, `place(flags: Flags)` modifying both fields. That fixture also needs a `flags` parameter, and the step names neither. Fix: name the fixture (TC-1045's, plus `Extra`, with `i1` binding `flags`), or use K2 with `place` modifying `placed` and contexts holding only `placed`. | spec/functional/FR-366-classify-check-and-evaluate-the-invariant-clauses-of-a-model-type.md:209; spec/test-cases/TC-1051-invariants-of-record-and-event-types-are-evaluated-once-for-each-reachable-value.md:69-75,112-117 |

## Dispositions

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 993a787fa1229ca708753d948b2a236d217449a3 |
| FND-002 | fixed | 993a787fa1229ca708753d948b2a236d217449a3 |
| FND-003 | fixed | 993a787fa1229ca708753d948b2a236d217449a3 |
| FND-004 | fixed | 993a787fa1229ca708753d948b2a236d217449a3 |
| FND-005 | fixed | 993a787fa1229ca708753d948b2a236d217449a3 |

Round 1 verification notes (reviewed at 993a787fa1229ca708753d948b2a236d217449a3):
- FND-001: quire-exact at the locked rev 37aa172 `numeric.rs` `order` makes
  three charges for integers: `OrderingOperands`, `OrderingArithmetic`, then
  `OrderingResultRetain` (line 163). The merged D01 test
  (`qsl-eval/tests/it/dispatch_calls.rs`) logs exactly these three after the
  call charges. The `Field` arm and literals charge nothing, so each value
  costs 3 charges. `discount` is first in key order. Denying charge 4 leaves one
  result, and denying charge 3 leaves none. Both are correct. (My round-0 count
  of 2 missed `result-retain`; the conclusion that the old row was wrong
  stands.)
- FND-003: `evaluate_clause` is now stated as unit-clause only, which matches
  `ClauseSetup::of`. AC-9's process truth values are right: the merged M16
  test evaluates a unit clause, `invariant NonNegativeAttempts using v on
  M::Fulfilment at current`, and gets `r1` true and `r2` false, located at
  `(s1, object r2)` with no steps. Its `process_document` also adds the
  package clause of the same name.
- FND-005: QSpec FR-207 key order compares the observation key first, then
  subject kind (object before invocation binding), then steps by kind and
  name, with index steps compared numerically. The labelled `s1` < `s2` gives
  exactly discount, history[0], history[1], quoted, total, then the parameter
  at `s2`. This is consistent.

Recorded, non-blocking (round 1):
- medium: For a machine snapshot with no context, FR-366 says it is
  `wrong-context` "as FR-106 check 6.10 refuses it". Check 6.10 is the
  `contexts`-on-a-`current`-snapshot rule. `wrong-context` comes from check 9,
  which FR-106 now says does not apply to `ValueInvariants`. Cite the check
  that applies, or say that check 9's context half applies.
- medium: AC-9 and TC-1051 step 11 expect `evaluate_clause("NonNegativeAttempts")`
  to return `UnknownClause`. This holds only if the step's unit declares no
  unit invariant of that name, but the merged M16 fixture declares one. State
  that the unit declares no such invariant.
- low: `ac:non-canonical-shape` warning at FR-366 line 204 (AC-4). The
  criterion asserts the outcome directly. The "with ..." preamble trips the
  heuristic, so it is not a defect.
- The earlier mediums are fixed: FR-056-AC-35 is now cited, FR-106 now states
  which checks apply to `ValueInvariants`, the shared meter is stated as
  deliberate, and `Undecided` is excluded.
