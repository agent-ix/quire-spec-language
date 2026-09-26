---
id: FR-107
title: "Evaluate a state clause over admitted observations at S6a"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-003
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-012
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-090
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-104
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-106
    type: traces_to
  - target: ix://agent-ix/quire-specification/FR-153
    type: depends_on
---
# FR-107: Evaluate a state clause over admitted observations at S6a

## Description

S6a SHALL evaluate one checked state clause (FR-104) over one
`AdmittedObservations` value (FR-106) and SHALL return FR-090's
`Evaluation`: `FamilyOutcome::Evaluated` carrying the kernel
`Outcome<Boolean>` (`Completed(true)`, `Completed(false)`, `Undefined`,
`Refused` or `Incomplete`), or `FamilyOutcome::FamilyEvaluated` for a
family-owned result, beside the location of the node that stopped it.

S6a is function-call only today. `S6aFamilyKind` has one variant, `Value`
(`qsl-eval/src/value/expression/s6a.rs:78-101`), and `CheckedPackageEvaluation`
has `call` and `evaluate` over one `ObjectEnvironment`
(`qsl-eval/src/value/expression/mod.rs:272-321`). `pre(..)` switches only the
population anchor that `allInstances` and `lookup` read (`Anchor { Post, Pre }`,
`evaluate.rs:284`, `:749-755`); a field read under `pre` still reads the one
object environment.

## Inputs

- A `CheckedPackage` holding the clause, and the clause's `QualifiedName`.
- `AdmittedObservations` for that clause (FR-106).
- A `Meter`.

## Outputs

- `Result<Evaluation, CallFailure>`: `Ok` with the `Evaluation`, or
  `CallFailure::Input(InputRefusal)` for a selection refused before S6a, or
  `CallFailure::Fault(InternalFault)` for an S6a invariant break.

## Behavior

- `S6aFamilyKind` SHALL gain `ProtocolClause`, matched with one arm and no
  `_` arm (ADR-012 §5.1 S1). The S6a seam's `ProtocolClause` arm SHALL make
  one call to the `ProtocolClause` `evaluate` hook.
- `CheckedPackageEvaluation` SHALL gain `evaluate_clause(&self, clause:
  &QualifiedName, observations: &AdmittedObservations, meter: &mut Meter)
  -> Result<Evaluation, CallFailure>`.
- If the name resolves to no state clause, then `evaluate_clause` SHALL
  return `CallFailure::Input(InputRefusal::UnknownClause)` without charging
  the meter. If the observations were admitted for another clause, then
  `evaluate_clause` SHALL return
  `CallFailure::Input(InputRefusal::ObservationsMismatch)` without charging
  the meter.
- The `ProtocolClause` evaluator SHALL read each model read in the
  observation S3 gave it (FR-104 "Observations of reads"): `current` for an
  invariant, `pre` for a precondition, `post` for a postcondition, and `pre`
  for reads that S3 placed under `pre(e)`. A reference keeps the observation
  it was read in, so a `deref` of it reads that observation.
- The evaluator SHALL evaluate `self` to the selected object's reference,
  `result` to the invocation's result value, and each operation parameter to
  its admitted value. These do not change under `pre`.
- The evaluator SHALL read field `f` of the referenced object in the read's
  observation for `self.f` and `deref(r).f`. Admission has closed every
  complete population (FR-106 check 8), so if a read meets a missing object,
  the evaluator SHALL return `CallFailure::Fault(InternalFault)`, never a
  truth value.
- The evaluator SHALL compare references by (universe, object type, key),
  ignoring the observation, so a pre and a post reference to one object are
  equal.
- The evaluator SHALL evaluate `reaches(a, b, edge)` and charge it exactly as
  QSpec `value-accounting.md` ("Model and graph evaluation") states: it
  evaluates `a` and `b`, charges `graph.expand` for `a` and enqueues it; for
  each dequeued node in discovery order and each target `t` of `edge` in the
  edge's own order (sequence order for a sequence edge), it charges one
  `graph.edge`, returns `true` when `t` is `b`, and otherwise charges
  `graph.expand` for an undiscovered `t` and enqueues it; an empty queue
  gives `false`; then it charges `graph.result-retain`. An absent optional
  edge or an empty sequence has no target. So an isolated `a` does not reach
  itself, and a self-loop or a cycle back to `a` does.
- The evaluator SHALL charge `model.deref` for each `deref(...)` and
  `model.navigate` for each field read, as `value-accounting.md` states.
  `quire-exact`'s `ChargePoint` has no `graph.*` or `model.*` charge point
  today, so this requirement adds them there.
- The evaluator SHALL skip the right operand of `and`, `or` and `implies`
  when the left decides the result, as `Value` does today.
- If the meter denies a charge, then the evaluator SHALL return
  `Outcome::Incomplete`, category incomplete, code `resource_exhausted`/
  `insufficient-next-charge`, and no truth value. A new call starts with the
  caller's fresh meter; no state survives a call.
- If a `pre(e)` is reached in a postcondition whose observations carry no pre
  observation, then the evaluator SHALL return `FamilyResult::Refused` with
  `wrong_snapshot`/`wrong-anchor` (FR-090, ADR-013 T-6). Admission makes this
  unreachable for a well-formed selection; the arm exists so the case has a
  typed result.
- The evaluator SHALL be deterministic: the same package, clause,
  observations and meter budget give the same `Evaluation` and the same
  charge log.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-107-AC-1 | Over admitted observations of the FR-108 corpus: `ParentOrder` is `Completed(true)` for healthy-parent and absent-parent and `Completed(false)` for violating-parent; `NoCycle` is `Completed(false)` for cycle and self-loop and `Completed(true)` for healthy-parent; `VersionUnchanged` is `Completed(true)` for unchanged-version and `Completed(false)` for changed-version. | Test (TC-466) |
| FR-107-AC-2 | In changed-version, `pre(self.versionNumber)` evaluates to 2 and `self.versionNumber` to 3; a postcondition `pre(present(self.parent) implies deref(value(self.parent)).versionNumber = 1)` over the same invocation is `Completed(true)`. | Test (TC-466) |
| FR-107-AC-3 | For the precondition `reaches(self, target, parent)` over the chain `a → b → c` (`c.parent` absent): with `self` `a` and `target` `c` it is `Completed(true)` and the `reaches` node's charge log is exactly `graph.expand` (a), `graph.edge` (a→b), `graph.expand` (b), `graph.edge` (b→c), `graph.result-retain`, that is 2 expansions; with `target` `a` and with `self` `c`, `target` `a` it is `Completed(false)`; over the self-loop `a → a` with `target` `a` it is `Completed(true)`. A meter that denies any one of the five charges of the first case makes it `Incomplete` with `resource_exhausted`. | Test (TC-466) |
| FR-107-AC-4 | healthy-parent with a meter budget of zero is `Incomplete` with `resource_exhausted`/`insufficient-next-charge` and no truth value; the next call with the default budget is `Completed(true)`. | Test (TC-467) |
| FR-107-AC-5 | `evaluate_clause` with a name that is a function, not a state clause, returns `CallFailure::Input(InputRefusal::UnknownClause)`; with observations admitted for `ParentOrder` but the name `NoCycle`, it returns `CallFailure::Input(InputRefusal::ObservationsMismatch)`; neither reaches S6a (the meter is uncharged). | Test (TC-467) |
| FR-107-AC-6 | Evaluating each corpus case twice gives equal `Evaluation`s and equal charge logs. With the S6a probe variant added, the build fails with E0004 at exactly the checked-in S6a seam list, whose `evaluate_declaration` match now has the `ProtocolClause` arm (FR-063). | Test (TC-467) |

## Dependencies

- FR-090 (the outcome shape; it needs no amendment: its S6a family kind has
  "one variant per family implementing `ReferenceEvaluation`", which already
  admits `ProtocolClause`), FR-104 (the checked clause and its read
  observations), FR-106 (the admitted observations), FR-062 and FR-063 (the
  S6a seam and probe).
- QSpec `state-contract.md` ("Operation anchors, aliases and captures",
  "Finite graph extension"), `value-accounting.md` ("Model and graph
  evaluation") and FR-153.
