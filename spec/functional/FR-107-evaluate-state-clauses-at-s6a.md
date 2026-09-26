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

- `Result<Evaluation, InternalFault>`.

## Behavior

- `S6aFamilyKind` SHALL gain `ProtocolClause`, matched with one arm and no
  `_` arm (ADR-012 §5.1 S1). Its arm SHALL make one call to the
  `ProtocolClause` `evaluate` hook.
- `CheckedPackageEvaluation` SHALL gain `evaluate_clause(&self, clause:
  &QualifiedName, observations: &AdmittedObservations, meter: &mut Meter)
  -> Result<Evaluation, CallFailure>`. A name that resolves to no state clause
  SHALL refuse `InputRefusal::UnknownClause` before S6a. Observations admitted
  for another clause SHALL refuse `InputRefusal::ObservationsMismatch` before
  S6a.
- The observation a read uses SHALL follow the clause kind: an invariant
  reads the current observation, a precondition reads the pre observation
  and a postcondition reads the post observation. Inside `pre(e)` in a
  postcondition every field read, `deref`, `reaches`, `allInstances` and
  `lookup` SHALL read the pre observation. `pre(pre(e))` reads the same pre
  observation.
- `self` SHALL evaluate to the selected object's reference. `result` SHALL
  evaluate to the invocation's result value, and each operation parameter to
  its admitted value. These do not change under `pre`.
- `self.f` and `deref(r).f` SHALL read field `f` of the referenced object in
  the observation in force. Admission has closed every complete population
  (FR-106 check 8), so a read never meets a missing object; if it does, S6a
  SHALL return `InternalFault`, never a truth value.
- Reference equality SHALL compare (universe, object type, key) and SHALL
  ignore the observation, so a pre and a post reference to one object are
  equal.
- `reaches(a, b, edge)` SHALL be true exactly when a path of one or more
  `edge` steps leads from `a` to `b` in the observation in force. It SHALL
  follow edges in declared order, SHALL test each reached identity against
  `b` before it suppresses a repeat expansion, SHALL expand each identity at
  most once, and SHALL charge the meter once per expansion. An absent
  optional edge or an empty collection has no step. So an isolated `a` does
  not reach itself, and a self-loop or a cycle back to `a` does.
- `and`, `or` and `implies` SHALL skip their right operand when the left
  decides the result, as `Value` does today.
- Meter exhaustion SHALL return `Outcome::Incomplete`, category incomplete,
  code `resource_exhausted`/`insufficient-next-charge`, and no truth value.
  A new call starts with the caller's fresh meter; no state survives a call.
- A `pre(e)` reached in a postcondition whose observations carry no pre
  observation SHALL return `FamilyResult::Refused` with `wrong_snapshot`/
  `wrong-anchor` (FR-090, ADR-013 T-6). Admission makes this unreachable for
  a well-formed selection; the arm exists so the case has a typed result.
- Evaluation SHALL be deterministic: the same package, clause, observations
  and meter budget give the same `Evaluation` and the same meter charges.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-107-AC-1 | Over admitted observations of the FR-108 corpus: `ParentOrder` is `Completed(true)` for healthy-parent and absent-parent and `Completed(false)` for violating-parent; `NoCycle` is `Completed(false)` for cycle and self-loop and `Completed(true)` for healthy-parent; `VersionUnchanged` is `Completed(true)` for unchanged-version and `Completed(false)` for changed-version. | Test (TC-466) |
| FR-107-AC-2 | In changed-version, `pre(self.versionNumber)` evaluates to 2 and `self.versionNumber` to 3; a postcondition `pre(present(self.parent) implies deref(value(self.parent)).versionNumber = 1)` over the same invocation is `Completed(true)`. | Test (TC-466) |
| FR-107-AC-3 | `reaches` over the three-object chain `a → b → c` is true for `(a, c)` and false for `(c, a)` and `(a, a)`; over the self-loop `a → a` it is true for `(a, a)`; with a meter budget of `k` charges it is `Incomplete` with `resource_exhausted` for every `k` below the expansions it needs and completes at that number. | Test (TC-466) |
| FR-107-AC-4 | healthy-parent with a meter budget of zero is `Incomplete` with `resource_exhausted`/`insufficient-next-charge` and no truth value; the next call with the default budget is `Completed(true)`. | Test (TC-467) |
| FR-107-AC-5 | `evaluate_clause` with a name that is a function, not a state clause, refuses `InputRefusal::UnknownClause`; with observations admitted for `ParentOrder` but the name `NoCycle`, it refuses `InputRefusal::ObservationsMismatch`; neither reaches S6a (the meter is uncharged). | Test (TC-467) |
| FR-107-AC-6 | Evaluating each corpus case twice gives equal `Evaluation`s and equal meter charge logs. With the S6a probe variant added, the build fails with E0004 at exactly the checked-in S6a seam list, whose `evaluate_declaration` match now has the `ProtocolClause` arm (FR-063). | Test (TC-467) |

## Dependencies

- FR-090 (the outcome shape), FR-104 (the checked clause), FR-106 (the
  admitted observations), FR-062 and FR-063 (the S6a seam and probe).
- QSpec `state-contract.md` ("Operation anchors, aliases and captures",
  "Finite graph extension") and FR-153.
