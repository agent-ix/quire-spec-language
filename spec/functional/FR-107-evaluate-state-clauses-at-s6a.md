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
  - target: ix://agent-ix/quire-specification/FR-207
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

- `evaluate_clause`: `Result<ClauseEvaluation, CallFailure>`: `Ok` with a
  `ClauseEvaluation` holding the `Evaluation` and what the evaluation
  recorded beside it (FR-265's decisions and stop reports, the receiver path
  and the read paths below), or `CallFailure::Input(InputRefusal)` for a
  selection refused before S6a, or `CallFailure::Fault(InternalFault)` for an
  S6a invariant break.
- `evaluate_clause_set`: `Result<ClauseSetEvaluation, CallFailure>`, below.

## Behavior

- `S6aFamilyKind` SHALL gain `ProtocolClause`, matched with one arm and no
  `_` arm (ADR-012 §5.1 S1). The S6a seam's `ProtocolClause` arm SHALL make
  one call to the `ProtocolClause` `evaluate` hook.
- `CheckedPackageEvaluation` SHALL gain `evaluate_clause(&self, clause:
  &QualifiedName, observations: &AdmittedObservations, meter: &mut Meter)
  -> Result<ClauseEvaluation, CallFailure>`.
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

### The receiver path

`ClauseEvaluation::receiver_path()` SHALL return the QSpec FR-207 runtime
value path of `self` under the clause's own observation, with no steps, or
none:

- If `self` has a declared object identity (QSpec FR-204), then the path's
  subject SHALL be that declared object identity, whether or not an
  invocation is selected.
- If `self` is a context with no declared object identity (a state machine's
  context, QSpec FR-208) and the admitted observations select an invocation,
  then the path's subject SHALL be that invocation's receiver binding.
- If `self` is such a context and no invocation is selected (a precondition
  selected by `PreCall`), then the path SHALL be none: there is no invocation
  to name, and QSpec FR-207 gives an absent subject no path.
- The path's observation SHALL be the clause's own observation (`current` for
  an invariant, `pre` for a precondition, `post` for a postcondition).

`AdmittedObservations` (FR-106) carries the selected invocation's identity,
whatever kind `self` is, or none; a receiver, parameter or result binding is
rooted at it.

### Read paths

`ClauseEvaluation::reads()` SHALL return the clause's reads in the order the
evaluator performed them. A read is the evaluation of a field or member
selection `e.f`, including `deref(e).f` and a selection through an implicit
dereference of a `Reference`. Every other node (a literal, an operator, a
function call, `value(e)`, `deref(e)`, `present(e)`, a relationship
navigation `e.end`) records no entry of its own. Each entry SHALL hold the
location of its selection node and the FR-207 runtime value path of the value
the selection read, or none.

- The evaluator SHALL record one entry for each evaluation of a selection
  node that is charged `model.navigate` as a field read: a node evaluated
  once per element of a quantifier records one entry per element; a node on
  a branch a short-circuit skips records none; a read whose charge the meter
  denies records none. The operand is evaluated, and so recorded, before the
  selection that consumes it, and a left operand before a right one.
- The root observation of an entry SHALL be the observation the read took
  place in (FR-104 "Observations of reads"): the clause's own observation, or
  the pre observation for a read under `pre(e)`. A path that follows a
  reference keeps the observation of that reference.
- The root subject SHALL be the first of these that applies to the selection's
  operand: `self` of an object receiver, the declared object identity of
  `self`; `self` of a context receiver, the invocation's receiver binding; a
  parameter `p`, the invocation's parameter binding `p`; `result`, the
  invocation's result binding; an element of a quantifier's domain, the
  stored or built location FR-265 gives that element (QSpec FR-207-AC-9 and
  AC-11). A selection whose operand has no such location (a literal, an
  arithmetic or function result, a navigation) has no path of its own, with
  the one exception below. A parameter, result or receiver binding with no
  selected invocation has none.
- The steps SHALL extend the operand's path by one step per selection: the
  `member` step (QSpec FR-207's member selection, FR-265-AC-7) for a
  selection from a declared object, `field` for a selection from a record
  value and for a context receiver's field (QSpec FR-208 "Locations"),
  `option_value` for each `value(e)` between the operand and the selection,
  and `reference` naming the referenced object's declared identity (QSpec
  FR-207-AC-10) for each dereference of a `Reference`, explicit or implicit,
  other than `self` of an object receiver, which is already the declared
  object identity.
- A selection on the result of a relationship navigation `e.end` (FR-363,
  FR-364) has the referenced object's declared object identity as its subject
  and the member step alone, because QSpec FR-207 has no step for a
  relationship end: the path locates the stored value, not the route that
  reached it.
- A path SHALL name no value the evaluation did not read: it holds exactly
  the steps of the selections evaluated.

### Clause sets

`CheckedPackageEvaluation` SHALL gain `evaluate_clause_set(&self, clauses:
&[(QualifiedName, &AdmittedObservations)], limits: ScalarLimits) ->
Result<ClauseSetEvaluation, CallFailure>`, the one place a set of clauses of
one invocation, one object or one state is evaluated and combined (FR-120's
conjunctions use it). Each clause is evaluated as `evaluate_clause` evaluates
it, with a fresh meter built from `limits` (FR-120).

- The result SHALL be `ClauseSetEvaluation { results: Vec<ClauseSetEntry>,
  outcome: ClauseSetOutcome }`. A `ClauseSetEntry` holds the clause's name
  and its `ClauseEvaluation`; there is one entry for each clause the set
  evaluated, and none for a clause it did not.
- The set SHALL evaluate its invariants, then its preconditions, then its
  postconditions, each group in ascending name order (UTF-8 bytes) whatever
  order the slice lists them in.
- The set SHALL evaluate its postconditions only when it evaluated every
  precondition it holds and every one is `Completed(true)`: a call whose
  precondition is not true is outside the operation's contract (FR-120, "An
  application is enabled when its effective precondition is `true`"), so its
  postconditions give no result. Invariants are never skipped.
- `outcome` SHALL be `Incomplete` when a clause evaluated to `Incomplete`; the
  set stops at that clause, which is its last entry. Otherwise it is
  `False` when any entry is `Completed(false)`, `Undecided` when none is and
  one or more are `Undefined` or `Refused`, and `True` otherwise, including
  for an empty set (FR-120's conjunction).
- A clause named twice in the slice SHALL be refused
  `CallFailure::Input(InputRefusal::DuplicateClause)` naming it. An unknown
  name and a mismatched `AdmittedObservations` are refused as
  `evaluate_clause` refuses them. A refused set returns no entry.
- A `CallFailure::Fault` from any clause SHALL be the set's result.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-107-AC-1 | Over admitted observations of the FR-108 corpus: `ParentOrder` is `Completed(true)` for healthy-parent and absent-parent and `Completed(false)` for violating-parent; `NoCycle` is `Completed(false)` for cycle and self-loop and `Completed(true)` for healthy-parent; `VersionUnchanged` is `Completed(true)` for unchanged-version and `Completed(false)` for changed-version. | Test (TC-466) |
| FR-107-AC-2 | In changed-version, `pre(self.versionNumber)` evaluates to 2 and `self.versionNumber` to 3; a postcondition `pre(present(self.parent) implies deref(value(self.parent)).versionNumber = 1)` over the same invocation is `Completed(true)`. | Test (TC-466) |
| FR-107-AC-3 | For the precondition `reaches(self, target, parent)` over the chain `a → b → c` (`c.parent` absent): with `self` `a` and `target` `c` it is `Completed(true)` and the `reaches` node's charge log is exactly `graph.expand` (a), `graph.edge` (a→b), `graph.expand` (b), `graph.edge` (b→c), `graph.result-retain`, that is 2 expansions; with `target` `a` and with `self` `c`, `target` `a` it is `Completed(false)`; over the self-loop `a → a` with `target` `a` it is `Completed(true)`. A meter that denies any one of the five charges of the first case makes it `Incomplete` with `resource_exhausted`. | Test (TC-466) |
| FR-107-AC-4 | healthy-parent with a meter budget of zero is `Incomplete` with `resource_exhausted`/`insufficient-next-charge` and no truth value; the next call with the default budget is `Completed(true)`. | Test (TC-467) |
| FR-107-AC-5 | `evaluate_clause` with a name that is a function, not a state clause, returns `CallFailure::Input(InputRefusal::UnknownClause)`; with observations admitted for `ParentOrder` but the name `NoCycle`, it returns `CallFailure::Input(InputRefusal::ObservationsMismatch)`; neither reaches S6a (the meter is uncharged). | Test (TC-467) |
| FR-107-AC-6 | Evaluating each corpus case twice gives equal `Evaluation`s and equal charge logs. With the S6a probe variant added, the build fails with E0004 at exactly the checked-in S6a seam list, whose `evaluate_declaration` match now has the `ProtocolClause` arm (FR-063). | Test (TC-467) |
| FR-107-AC-7 | `receiver_path()` of `ParentOrder` over healthy-parent is rooted at the observation `ix://example/config-version/current/healthy-parent` and the declared object identity of `child`, with no steps; of the state machine invariant `Consistent` over an invocation `i1` it is rooted at `q1` and `i1`'s receiver binding, with no steps; of `PlaceRequires` over a pre-call observation it is none. | Test (TC-1040) |
| FR-107-AC-8 | `reads()` of `ParentOrder` over healthy-parent holds four entries in this order: the two `self.parent` selections, `deref(value(self.parent)).versionNumber`, `self.versionNumber`; over absent-parent it holds the one `self.parent` entry, and with a work budget of zero it is empty. | Test (TC-1040) |
| FR-107-AC-9 | The paths of those four entries are: `[member parent]` twice, `[member parent, option_value, reference root, member versionNumber]` and `[member versionNumber]`, all rooted at the declared object identity of `child`; and `VersionUnchanged` over changed-version has two entries, both `[member versionNumber]` of `child`, the first rooted at the post observation and the second, read under `pre`, at the pre observation. | Test (TC-1040) |
| FR-107-AC-10 | In `ProbeOrdered`, `target.versionNumber` over an invocation is located at the parameter binding `target` of that invocation with steps `[reference b, member versionNumber]`, and `self.versionNumber` at the declared object identity of `a`; over a pre-call observation the first has no path and the second the same path as before. | Test (TC-1040) |
| FR-107-AC-11 | `Consistent`'s reads of `self.placed` and `self.cancelled` are located at the receiver binding of `i1` with the steps `[field placed]` and `[field cancelled]`, `FlagsPlaced`'s read of `flags.placed` at the parameter binding `flags` of `i1` with `[field placed]`; the two `placed` paths compare unequal, the parameter path orders first, and their wire forms are the objects TC-1040 lists. | Test (TC-1040) |
| FR-107-AC-12 | The elements of the list `[value(self.parent), self]` built inside `GuardedAll` are read at the built-list subject with the steps `[index 0, reference root, member versionNumber]` and `[index 1, reference mid, member versionNumber]`; a selection on `self.home`, a navigation, is located at the declared object identity of the tank `t1` with the step `[member level]` and the navigation records no entry. | Test (TC-1040) |
| FR-107-AC-13 | `evaluate_clause_set` over `UpdateAllowed` and `UpdateRaises` gives two results, both `Completed(true)` and outcome `True`, for a call from version 1 to 2, and exactly one result, `UpdateAllowed` `Completed(false)`, and outcome `False` for a call from version 3 to 4; the slice order does not change a result. | Test (TC-1039) |
| FR-107-AC-14 | With `UpdateNotMax` added, the call from 3 to 4 gives the two results `UpdateAllowed` `Completed(false)` and `UpdateNotMax` `Completed(true)`, in that order, and no result for `UpdateRaises`; with the invariant `NoCycle` over its own observation added instead, the results are `NoCycle` `Completed(true)` then `UpdateAllowed` `Completed(false)`, and still none for `UpdateRaises`. | Test (TC-1039) |
| FR-107-AC-15 | With `limits` whose work budget is zero the call from 1 to 2 gives one result, `UpdateAllowed` `Incomplete`, and outcome `Incomplete`; an empty set gives no result and outcome `True`; the same set evaluated twice gives equal results. | Test (TC-1039) |
| FR-107-AC-16 | A set naming `UpdateAllowed` twice is refused `InputRefusal::DuplicateClause`; a set naming the function `sameIdentity` is refused `InputRefusal::UnknownClause`; a set pairing `UpdateAllowed`'s name with observations admitted for `UpdateRaises` is refused `InputRefusal::ObservationsMismatch`; none returns a result. | Test (TC-1039) |

## Dependencies

- FR-090 (the outcome shape; it needs no amendment: its S6a family kind has
  "one variant per family implementing `ReferenceEvaluation`", which already
  admits `ProtocolClause`), FR-104 (the checked clause and its read
  observations), FR-106 (the admitted observations), FR-062 and FR-063 (the
  S6a seam and probe).
- FR-265 (the decisions and stop reports `ClauseEvaluation` carries),
  FR-120 (the conjunction and enabling rules the set shares), FR-363 and
  FR-364 (navigation). QSpec FR-207 (the runtime value path: subjects,
  steps, absence), FR-208 ("Locations", AC-10) and FR-204.
- QSpec `state-contract.md` ("Operation anchors, aliases and captures",
  "Finite graph extension"), `value-accounting.md` ("Model and graph
  evaluation") and FR-153.
