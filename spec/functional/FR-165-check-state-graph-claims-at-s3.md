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
fairness constraint, at the offending span; a placement `graph-predicate`
does not recognise is a syntax error.

## Inputs

- Parsed state-graph claim syntax (S1), in the spellings QSpec's shared
  grammar gives the three forms. The `over` parameter is optional in all
  three, and a predicate is read through `graph-predicate` (see Parsing).
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

### Parsing (S1)

- S1 SHALL parse each of the three forms with the `over (x: T)` parameter
  present or absent. A claim without `over` parses to a claim with no `over`
  binding.
- S1 SHALL parse each predicate of a state-graph claim (a body, a `from`
  operand, a `to` operand) through the `graph-predicate` production of
  QSpec's shared grammar (state-graph section), and a body's leading
  fairness constraints through `graph-body`. The production is recognition
  only and is owned by QSpec; this requirement does not restate it. QSpec
  FR-390 and its TC-330 vectors SG-R09 to SG-R18 are the reference for what
  it recognises.
- S1 SHALL NOT refuse a temporal operator, with or without an interval, that
  `graph-predicate` recognises: one standing as an operand of
  `and`, `or` or `implies`, of a binary temporal operator, of a unary
  operator, of a parenthesised group or of a branch of an `if` or the body
  of a `let` (`c.n = 1 and eventually c.n = 2`,
  `not eventually c.n = 2`, `(eventually c.n = 2)`, `c.a until c.b`,
  and one in a `from` or `to` operand). It SHALL NOT refuse a fairness
  constraint at the start of a body. S3 refuses these (see Predicates).
- S1 SHALL refuse with `invalid_syntax` everything `graph-predicate` does
  not recognise, including:
  - a temporal operator in an `if` condition, at the operator;
  - a temporal operator in a `let` binding, at the operator;
  - a temporal operator in an operand of a primary or a comparison, at the
    operator (a call argument, a quantifier body such as `exists(d in
    c.items: eventually d.n = 2)`, or `c.x = eventually c.y`);
  - a fairness constraint after the predicate of a body, inside a `from`
    operand or inside a `to` operand, at the `;` that follows the predicate;
  - a fairness constraint after a `unique path` clause, which has no body,
    at the `fair` token that follows the clause's `;`;
  - a second binary temporal operator in one `graph-relation`
    (`c.a until c.b until c.c`), at the second operator. A chain is written
    with parentheses, `(c.a until c.b) until c.c`, and parses.
- Where a predicate holds both a name that does not resolve and a temporal
  operator S1 recognised, S3 SHALL resolve names first: it refuses
  `missing-name` at the name before it considers the operator.
- A phrase that derives as an `expr` is read as that `expr`, so
  `not c.n = 2` is `(not c.n) = 2` and reaches the checker as an ordinary
  expression.

### Forms

- S2 SHALL build one state-graph claim form for each parsed `possible`,
  `always possible` and `unique path` claim, holding its name, profile
  reference, `over` parameter and predicates.
- S3 SHALL check `possible` to `Possible`, `always possible` to
  `AlwaysPossible`, and `unique path` to `UniquePath`.
- An `always possible` claim written without `from` SHALL check with `from`
  as the constant `true` predicate.
- S3 SHALL record a claim's `over` binding (`Some` when written, `None`
  when absent) and SHALL NOT instantiate it. A population universe is a
  request-time input (`ModelSubject.universes`, FR-125), not an S3 input, so
  FR-125 instantiates the claim: one instance per object of the parameter's
  universe, or one instance when the claim has no `over` binding.
- The form and every predicate SHALL be part of the claim's checked
  identity, so two claims that differ only in form have different node
  identities.

### Predicates

- S3 SHALL check each predicate (`target`, `from`, `to`) as a Boolean state
  clause under the unit's value profile, through the clause checker that
  invariants use (FR-104).
- If a predicate reads an operation anchor, a result or a pre-state, then S3
  SHALL refuse `unsupported_construct`/`expression-form` located at the read
  (ADR-022 GM-3). These are the forms that name an operation context.
- A claim has no operation context, so S3 resolves a predicate's names
  against the `over` binding and the unit's state alone. If a predicate
  names an operation parameter, which no claim scope binds, then S3 SHALL
  refuse `missing-name` located at the name. S3 is the only stage that
  refuses a parameter read.
- If a predicate contains a temporal operator that S1 recognised, then S3
  SHALL refuse `unsupported_construct`/`expression-form` located at the
  operator; where a predicate holds several, at the first in source order.
  A placement S1 does not recognise never reaches S3 (see Parsing).
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
| FR-165-AC-1 | Over ADR-022 §7's units: `ReachesTwo` and `ReachesThree` check to `Possible`, each with its `over` binding recorded; `CanStillWin` without `from` checks to `AlwaysPossible` with `from` the constant `true`, and with `from (x.phase != Lost)` to `AlwaysPossible` with that predicate; `InOneWay` checks to `UniquePath`. Each records (`temporal-satisfaction`, `Unbounded`) with its form beside it. `possible P` and `always possible P` over the same `P` have different node identities. | Test (TC-590) |
| FR-165-AC-2 | Refusals, each `unsupported_construct`/`expression-form` at the named span, with no checked claim: `possible` whose predicate reads `pre(c.versionNumber)` (at the read); `possible` whose predicate is `eventually c.versionNumber = 2` (at `eventually`); `always possible` carrying `fair weak attemptUpdate` (at the constraint). | Test (TC-590) |
| FR-165-AC-3 | A request whose only items are `CanStillWin` and `InOneWay` carries one `DeadlockFreedom` item for each of their two subjects; a request with `ReachesTwo` and a temporal claim over the same subject carries one; the §7.2 game with `terminal any` carries none. | Test (TC-590) |
| FR-165-AC-4 | S1 parses `possible Anything using v { true }` (no `over`) to a claim with no `over` binding. Over `over (c: Config::ConfigVersion)`, S1 parses with no refusal the bodies `c.versionNumber = 2 and eventually c.versionNumber = 3`, `not eventually c.versionNumber = 2`, `not eventually[0,5] c.versionNumber = 2`, `eventually[1,*] c.versionNumber = 2`, `(c.versionNumber = 1 until c.versionNumber = 2)`, `(c.versionNumber = 1 until c.versionNumber = 2) until c.versionNumber = 3`, `if c.versionNumber = 1 then eventually c.versionNumber = 2 else c.versionNumber = 3`, `let n = c.versionNumber in eventually n = 2`, and `(eventually c.versionNumber = 2)`; `always possible` with `from (always c.versionNumber = 1)`; `unique path` with `to (eventually c.versionNumber = 2)`; and a body led by `fair weak Config::ConfigVersion::attemptUpdate;`. S3 checks the first with `over` recorded as `None` and refuses each other claim `unsupported_construct`/`expression-form`, with no checked claim and no profile refusal, at the first temporal operator in source order or at the constraint. | Test (TC-590) |
| FR-165-AC-5 | Over a fixture unit whose operation `setVersion` takes parameter `newVersion: Int`, S3 refuses a `possible` claim whose predicate is `c.versionNumber = newVersion` with `missing-name` at `newVersion`, not `expression-form`; the same claim with `pre(c.versionNumber)` still refuses `expression-form`, with no checked claim. | Test (TC-590) |
| FR-165-AC-6 | Over the same `over (c: Config::ConfigVersion)`, S1 refuses `invalid_syntax`, with no claim, at the named token: `{ if eventually c.versionNumber = 1 then c.versionNumber = 2 else c.versionNumber = 3 }` and `{ let n = eventually c.versionNumber = 1 in n = 2 }` at `eventually`; `{ exists(d in c.versionNumber: eventually d = 2) }`, `{ f(c.versionNumber, eventually c.versionNumber = 1) }` and `{ c.versionNumber = eventually c.versionNumber = 2 }` at `eventually`; `{ c.versionNumber = 1; fair weak Config::ConfigVersion::attemptUpdate }` and `from (c.versionNumber = 1; fair weak Config::ConfigVersion::attemptUpdate)` at the `;` after the predicate; `unique path ... to (c.versionNumber = 2); fair weak Config::ConfigVersion::attemptUpdate` at `fair`; `{ c.versionNumber = 1 until c.versionNumber = 2 until c.versionNumber = 3 }` at the second `until`. A claim with an unknown name and a recognised operator, `{ eventually c.versionNumber = newName }`, refuses `missing-name` at `newName`, not `expression-form`. | Test (TC-590) |

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

- ADR-022. QSpec half: QSpec FR-390 (Linear STD-135; ADR-022 QS-1, QS-8).
