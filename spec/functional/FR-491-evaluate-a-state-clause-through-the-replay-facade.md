---
id: FR-491
title: "Evaluate a state clause at an observation through the replay facade"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-106
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-107
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-109
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-060
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: traces_to
---
# FR-491: Evaluate a state clause at an observation through the replay facade

## Description

A backend that checks a state clause's truth at one observation, for example
CG's precondition oracle, must not name `qsl_replay::spine`: ADR-011 FB-05
lets CG depend on the layer-6 `replay` facade only, and T-12 rule (a)
(FR-060 T12-A) refuses any CG reference to `spine`. `spine::run_clause`
(FR-109) is the evaluation, but it returns a report built from spine types,
and it is not reachable from the crate root.

This requirement adds the replay-owned entry `qsl_replay::evaluate_clause`
beside `replay_state_clause` (FR-122). It runs FR-109's `Clause` selection
(FR-106 admission, then FR-107 evaluation) and returns one typed result.

## Inputs

- A `ClauseEvaluationRequest`: the unit (source identity, display path and
  bytes), the domain package documents it selects, its dependency input, the
  authored state clause's declared name, the spine and accounting limits and
  one `ClauseObservation`.
- A `ClauseObservation` is of the clause's own kind (FR-106 check 2):
  `Current` (snapshot document bytes, anchor, self object) for an invariant
  or constraint, `PreCall` (pre snapshot document bytes, self object,
  parameter values) for a precondition.

## Outputs

`ClauseEvaluation`:

- `Completed(bool)`: the clause evaluated; `true` when it holds at the
  observation, `false` when it does not.
- `Undefined { reason }`: the clause has no value at the observation.
- `Refused(ClauseRefusal)`: the request was refused before a value was
  computed: `EmptySource`, `UnreadableObservation`, `MissingClause`, or
  `Coded` (stage, category, catalog code and cause) for a compile, admission
  or evaluation refusal, including an observation of the wrong kind
  (`wrong_snapshot`/`wrong-observation`); or `Fault` for a broken invariant.
- `Incomplete(ClauseIncomplete)`: `Accounting` (the exhausted counter) or
  `Observation` (evidence the clause reads is missing at admission).

## Behavior

- The entry SHALL select the clause by name and the observation by its kind,
  admit it by FR-106 and evaluate it by FR-107, as FR-109's `Clause`
  selection does, and SHALL NOT restate either.
- The entry SHALL NOT evaluate a precondition at a current observation or an
  invariant at a pre-call observation; the mismatch is `Refused`.
- No input, type or result of the entry SHALL record or compare a digest,
  hash, revision, seal, lock or pin. The request supplies document bytes, not
  document labels; the result carries no identity of a package, source or
  document.
- The root SHALL export the entry and its types so a consumer does not name
  `qsl_replay::spine`.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-491-AC-1 | Over the `example/config-version` unit, the invariant `ParentOrder` at a `Current` snapshot where the parent's version is below the child's completes `true`, and where it is not completes `false`. | Test (TC-1274) |
| FR-491-AC-2 | The precondition `ParentPresent` at a `PreCall` snapshot completes `true` for an object with a parent and `false` for one without. | Test (TC-1274) |
| FR-491-AC-3 | `ParentOrder` at a `PreCall` observation and `ParentPresent` at a `Current` observation each return `Refused(Coded)` with code `wrong_snapshot` and cause `wrong-observation`. | Test (TC-1274) |
| FR-491-AC-4 | An S6a `Undefined` outcome of the clause settles as `Undefined` carrying the reason's tabled spelling, and a completed Boolean settles as `Completed`. | Test (TC-1274) |
| FR-491-AC-5 | A clause name the unit does not declare returns `Refused(MissingClause)`; bytes that are not a snapshot document return `Refused(UnreadableObservation)`; accounting `work_units = 0` returns `Incomplete(Accounting)`. | Test (TC-1274) |
| FR-491-AC-6 | A consumer reaches the entry and every type of its request and result through the `qsl_replay` root alone, without naming `qsl_replay::spine`. | Test (TC-1274) |

## Dependencies

FR-106, FR-107, FR-109, FR-122 (the facade pattern), FR-060 (T12), ADR-011.
