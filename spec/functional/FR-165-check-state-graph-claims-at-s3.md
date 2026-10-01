---
id: FR-165
title: "Check state-graph claims at S3 and record their form"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-020
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-014
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-016
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-018
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-022
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-057
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-104
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-124
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-125
    type: depends_on
---
# FR-165: Check state-graph claims at S3 and record their form

## Description

The S2 value form builder SHALL build a form for each state-graph claim, and
the S3 check SHALL check it into one of three forms (ADR-022 SG-1 to SG-3):
`possible`, `always possible` with an optional `from`, and `unique path`
with `from` and `to`. S3 SHALL check every predicate of the claim as a state
predicate, refuse a predicate that reads anything other than the state,
refuse a fairness constraint, and record the claim's form beside its
requirement record (ADR-022 SG-4). The request writer SHALL add the
deadlock-freedom item for the subjects of state-graph items (ADR-022 GM-8).

## Use case

A verification operator writes `possible ReachesTwo using v over (c:
Config::ConfigVersion) { c.versionNumber = 2 }` and `always possible
CanStillWin using v over (x: G::Game) from (x.phase != Lost) { x.phase =
Won }`. The checker admits both, records which form each is, and refuses a
predicate that reads a pre-state, uses a temporal operator or carries a
fairness constraint, at the offending span.

## Inputs

- Parsed state-graph claim syntax (S1), in the spellings QSpec's shared
  grammar gives the three forms.
- The admitted domain packages and the unit's state models (FR-103).
- For the request writer: the request's temporal and state-graph items with
  their model subjects (FR-125).

## Outputs

```rust
pub struct CheckedStateGraphClaim {
    pub form: StateGraphForm,
    pub over: Option<OverBinding>,          // ADR-018 QS-5, as FR-125 reads it
}

pub enum StateGraphForm {
    Possible { target: CheckedStatePredicate },                       // SG-1
    AlwaysPossible { from: CheckedStatePredicate,                     // SG-2
                     target: CheckedStatePredicate },
    UniquePath { from: CheckedStatePredicate,                         // SG-3
                 to: CheckedStatePredicate },
}
```

- The claim's `Requirements{kind: temporal-satisfaction, extent:
  Unbounded{domains}}`, with `domains` naming the claim, and its
  `StateGraphForm` recorded beside it.
- A typed `CheckRefusal` with a span, and no checked claim, on refusal.
- For the request writer: the request's deadlock-freedom items.

## Behavior

### Forms

- S2 SHALL build one state-graph claim form for each parsed `possible`,
  `always possible` and `unique path` claim, holding its name, profile
  reference, `over` parameter and predicates.
- S3 SHALL check `possible` to `Possible`, `always possible` to
  `AlwaysPossible`, and `unique path` to `UniquePath`.
- An `always possible` claim written without `from` SHALL check with `from`
  as the constant `true` predicate.
- A claim with an `over` parameter SHALL check to one instance per object of
  the parameter's population universe, as FR-125 instantiates a temporal
  clause. A claim with no `over` parameter has one instance.
- The form and every predicate SHALL be part of the claim's checked
  identity, so two claims that differ only in form have different node
  identities.

### Predicates

- S3 SHALL check each predicate (`target`, `from`, `to`) as a Boolean state
  clause under the unit's value profile, through the clause checker that
  invariants use (FR-104).
- If a predicate reads an operation anchor, a parameter, a result or a
  pre-state, then S3 SHALL refuse `unsupported_construct`/`expression-form`
  located at the read (ADR-022 GM-3).
- If a predicate contains a temporal operator, then S3 SHALL refuse
  `unsupported_construct`/`expression-form` located at the operator.
- If the claim carries a fairness constraint, then S3 SHALL refuse
  `unsupported_construct`/`expression-form` located at the constraint
  (ADR-022 GM-7).

### Requirement record

- S3 SHALL record each checked claim as `Requirements{kind:
  temporal-satisfaction, extent: Unbounded{domains}}` (FR-057, ADR-014 A-3),
  with its `StateGraphForm` beside the record, so the claim routes on the
  existing kind and a candidate's arm reads the form.
- A state-graph claim SHALL carry no temporal profile, clock, activation or
  property form.

### Deadlock-freedom item

- The request writer SHALL add one `DeadlockFreedom` item (FR-124) per
  distinct model subject among the request's temporal items and state-graph
  items together. A temporal item and a state-graph item over equal subjects
  share one deadlock-freedom item.
- The request writer SHALL add none for a subject whose state model declares
  `terminal any` (FR-124).

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-165-AC-1 | Over ADR-022 §7's units: `ReachesTwo` and `ReachesThree` check to `Possible`, each with two instances over universe `{a, b}`; `CanStillWin` without `from` checks to `AlwaysPossible` with `from` the constant `true`, and with `from (x.phase != Lost)` to `AlwaysPossible` with that predicate; `InOneWay` checks to `UniquePath`. Each records (`temporal-satisfaction`, `Unbounded`) with its form beside it. `possible P` and `always possible P` over the same `P` have different node identities. | Test (TC-590) |
| FR-165-AC-2 | Refusals, each `unsupported_construct`/`expression-form` at the named span, with no checked claim: `possible` whose predicate reads `pre(c.versionNumber)` (at the read); `possible` whose predicate is `eventually c.versionNumber = 2` (at `eventually`); `always possible` carrying `fair weak attemptUpdate` (at the constraint). | Test (TC-590) |
| FR-165-AC-3 | A request whose only items are `CanStillWin` and `InOneWay` carries one `DeadlockFreedom` item for each of their two subjects; a request with `ReachesTwo` and a temporal claim over the same subject carries one; the §7.2 game with `terminal any` carries none. | Test (TC-590) |

## Dependencies

- ADR-022 §1 SG-1 to SG-5, §2 GM-3, GM-7 and GM-8; ADR-018 QS-5 (the `over`
  binding); ADR-014 A-3 (requirement records).
- [FR-104](FR-104-check-state-clauses.md) (the state clause
  checker), [FR-124](FR-124-declare-intended-terminal-states-and-derive-deadlock-freedom.md)
  (deadlock-freedom item), [FR-125](FR-125-read-a-model-subject-s-behaviours-as-temporal-traces.md)
  (model subject and `over` instantiation),
  [FR-057](FR-057-admit-shared-capability-kinds.md)
  (`temporal-satisfaction`).
- QSpec owns the surface grammar of the three forms and their refusals
  (ADR-022 QS-1); the spellings here follow ADR-022 §7.

## References

- ADR-022. QSpec half: Linear STD-135 (ADR-022 QS-1, QS-8).
