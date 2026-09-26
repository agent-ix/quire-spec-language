---
id: FR-104
title: "Check state clauses at S3 and record their requirements"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-003
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-012
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-014
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-062
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-057
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-102
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-103
    type: traces_to
  - target: ix://agent-ix/quire-specification/FR-153
    type: depends_on
---
# FR-104: Check state clauses at S3 and record their requirements

## Description

When S3 receives a `StateClauseForm` (FR-102), the `ProtocolClause` family
`check` SHALL resolve its profile alias, its context type and, for a `pre` or
`post` clause, its operation; SHALL type its body as a Boolean under its
clause kind, with `self`, `result`, the operation's parameters and `pre(e)`
bound as QSpec's state contract states; and SHALL produce one checked state
clause with its identity, or a refusal with a catalog code. Its
`requirements` hook SHALL return one `operation-contract` record per clause
and per frame (ADR-012 §2, §15.7; FR-057).

S3 has a clause-kind checker today, `CheckedGraph::check_clause_expression`
(`qsl-semantics/src/check/mod.rs:1266`), but it takes explicit parameters and
has no clause context: no receiver, no operation and no `result`
(`check/check.rs:696-715`). Field access on a model type exists only as
`deref(r).f` (the `Attribute` node, `check/check/typing.rs:663-677`), and
`pre(e)` accepts only an operand that contains `allInstances`, `lookup` or
`pre` (`contains_pre_eligible_read`, `check/check.rs:774-788`).

## Inputs

- A `StateClauseForm`; the assembled `PackageDeclarations` with the
  operations FR-103 admits; `CheckContext` (ADR-012 §2).

## Outputs

- A checked state clause: its kind, its context object type, its operation
  (for `pre` and `post`), its checked Boolean body, its node identity and its
  `claim` occurrence (ADR-013 O-07, O-09), recorded in the `CheckedGraph`
  under its declared name.
- One `Requirements` record per clause, and one per frame of an operation
  that at least one `pre` or `post` clause of the unit names.
- Or a refusal with a family cause and catalog code, or
  `StageFailure::Limit`.

## Behavior

### Resolution

- The `using` alias SHALL resolve to one of the unit's profile selections,
  exactly as a function's does (FR-091); an unknown alias refuses as it does
  for a function.
- `M` SHALL name a `model` declaration of the unit and `T` an object type of
  its package. Otherwise the check SHALL refuse
  `missing_declaration`/`missing-name` at the name.
- For `pre` and `post`, `op` SHALL name an operation of `M::T`'s effective
  view (FR-103). Otherwise the check SHALL refuse
  `missing_declaration`/`missing-name` at `op`.
- Two state clauses of one unit with the same name SHALL refuse at the second
  with the code a duplicate function name gets.

### Typing

- The body SHALL check against `Boolean`. A body of another type SHALL refuse
  `ill_typed`/`non-boolean-root` at the body.
- `self` SHALL check as `Reference<M::T>`, the context object. A field read
  `self.f` SHALL check as the `Attribute` node `deref(self).f` produces,
  typed by `f`'s declared type in the effective view. `self` outside a state
  clause SHALL refuse `missing_declaration`/`missing-name`.
- `result` SHALL be legal only in a `post` clause of an operation that
  declares a result, and SHALL check as that result's type. Elsewhere it SHALL
  refuse `wrong_snapshot`/`wrong-anchor` at `result`, naming the clause kind
  and the operation.
- Each parameter of the operation SHALL be in scope by name in a `pre` or
  `post` body, typed as FR-103 declares it. No parameter is in scope in an
  invariant.
- `pre(e)` SHALL stay legal only in a `post` clause (the existing
  `ForbiddenPreRead` rule). A field read of `self`, or of a reference reached
  from `self`, SHALL count as an eligible state read, beside `allInstances`,
  `lookup` and `pre`. `pre(e)` whose operand reads only parameters,
  literals or `result` SHALL refuse `wrong_snapshot`/`forbidden-pre-read`
  (QSpec state contract, "Operation anchors, aliases and captures").
- `reaches(a, b, edge)`, in a state clause or a function body, SHALL require `a` and `b` to be `Reference<M::T>` of
  one object type and `edge` to name a field of `T` whose type is
  `Reference<M::T>`, `Option<Reference<M::T>>` or a collection of
  `Reference<M::T>`. It checks as `Boolean`. Anything else SHALL refuse
  `ill_typed`/`operator-ineligible` at the `reaches`.
- Equality of two `Reference<M::T>` values SHALL check as identity equality,
  as it does for function parameters today.
- The body SHALL pass the static definedness check (`CheckMode::Linked`) with
  the clause's guards, so `value(self.parent)` under
  `present(self.parent) implies` checks and an unguarded `value(self.parent)`
  refuses with the definedness cause it has today.

### Requirements

- Each clause SHALL yield one `Requirements` with capability kind
  `operation-contract`, keyed by the clause's `claim` occurrence (ADR-012
  §13.5).
- Each frame of an operation that a `pre` or `post` clause of the unit names
  SHALL yield one `operation-contract` record, keyed by the frame node's own
  occurrence. An operation no clause names yields no frame record.
- Each record's extent SHALL follow ADR-014 §4 over the clause's `self`,
  `result` and parameter types. A `Reference` is not an unbounded domain in
  that table, so a clause whose reachable field types are all bounded is
  `Bounded`.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-104-AC-1 | Over the ConfigVersion package (FR-103-AC-1), `ParentOrder`, `NoCycle` (`not reaches(self, self, parent)`) and `post VersionUnchanged using v on Config::ConfigVersion::attemptUpdate { self.versionNumber = pre(self.versionNumber) }` each check, with kind `Invariant`, `Invariant` and `Postcondition`; `self` is `Reference<Config::ConfigVersion>`, `self.versionNumber` is `Int[0, 1000]` and `self.parent` is `Option<Reference<Config::ConfigVersion>>`. | Test (TC-459) |
| FR-104-AC-2 | `post R using v on Config::ConfigVersion::attemptUpdate { result }` checks with `result: Boolean`. `result` in an invariant, and in a `pre` clause of `attemptUpdate`, each refuse `wrong_snapshot`/`wrong-anchor` at `result`. | Test (TC-459) |
| FR-104-AC-3 | `on Config::Missing` and `on Config::ConfigVersion::missing` refuse `missing_declaration`/`missing-name` at the missing name; an invariant body `self.versionNumber` refuses `ill_typed`/`non-boolean-root`; a second clause named `ParentOrder` refuses as a duplicate function name does. | Test (TC-460) |
| FR-104-AC-4 | `pre(self.versionNumber)` in an invariant refuses `wrong_snapshot`/`forbidden-pre-read`; `pre(result)` in a postcondition refuses `wrong_snapshot`/`forbidden-pre-read`; `reaches(self, self, versionNumber)` refuses `ill_typed`/`operator-ineligible`; an unguarded `deref(value(self.parent)).versionNumber < 5` refuses with the definedness cause. | Test (TC-460) |
| FR-104-AC-5 | The ConfigVersion unit with `ParentOrder`, `NoCycle` and `VersionUnchanged` yields exactly four `operation-contract` records: one per clause, keyed by its `claim` occurrence, and one for `attemptUpdate`'s frame, keyed by the frame node's occurrence. Each has extent `Bounded`. The same unit without `VersionUnchanged` yields exactly two, with no frame record. | Test (TC-461) |
| FR-104-AC-6 | Checking the same unit twice, and checking it with its clauses in another order, gives each clause the same node identity and the same requirement record keys. Two clauses with equal bodies and different names share a node identity and differ in their `claim` occurrence. | Test (TC-461) |

## Dependencies

- FR-102 (forms), FR-103 (operations), FR-062 and FR-097 (requirement
  records and extent), FR-088 (clause identity), FR-057 (the
  `operation-contract` kind).
- QSpec `state-contract.md` and FR-153 (the anchor table and `pre`).
- The code change waits for the other lane's current work in `check/`
  (QSL-273 ticket text).
