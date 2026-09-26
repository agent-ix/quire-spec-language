---
id: FR-102
title: "Build state clause forms and the self, result and reaches expressions at S2"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-003
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-012
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-067
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-091
    type: traces_to
  - target: ix://agent-ix/quire-specification/FR-019
    type: depends_on
---
# FR-102: Build state clause forms and the self, result and reaches expressions at S2

## Description

When a `1-draft` unit declares an `invariant`, `pre` or `post` state clause,
S2 (`qsl-forms`) SHALL build one `ProtocolClause` state clause form for it
through the closed leading-token-kind entry table (ADR-012 §3, §4.3, §15.3).
When a clause body or any other expression uses `self`, `result` or
`reaches(a, b, edge)`, S2 SHALL build an `Expression` variant for it instead
of refusing it as an unrepresented construct.

Today S2 refuses all of these. `LeadingTokenKind` has `Function`, `Type`,
`Record` and `Tuple` only (`qsl-forms/src/dispatch.rs:156-170`), a unit
beginning `invariant` refuses `NoDispatchEntry` (`dispatch.rs:671-682`), and
the expression builder refuses `self`, `result` and `reaches` as
`UnrepresentedConstruct` (`qsl-forms/src/value.rs:1039-1041`). S1 already
parses all of them (`qsl-cst/src/grammar.rs:566-591`, `:864-865`,
`:905-914`).

## Inputs

- A recovery-free `LosslessCst` of a `1-draft` unit (ADR-011 E2).
- `FormsLimits`.

## Outputs

- `DeclarationForm::StateClause(Box<StateClauseForm>)`, where
  `StateClauseForm` holds:
  - `kind`: `StateClauseKind { Invariant, Precondition, Postcondition }`;
  - `name`: the declared identifier and its span;
  - `profile`: the `using` alias and its span;
  - `context`: the `model-name` (`M::T`) and its span;
  - `operation`: the operation member name and its span for a `pre` or
    `post` clause, and none for an invariant;
  - `body`: the block's `Expression` and its `DeclarationSpans`.
- `Expression::SelfRef`, `Expression::Result` and
  `Expression::Reaches { source, target, edge }`, where `edge` is the
  member name and its span.

## Behavior

- `LeadingTokenKind` SHALL gain `Invariant`, `Pre` and `Post`. Each SHALL
  select the one production `protocol_clause::state_clause`, and the
  `dispatch` arm SHALL make exactly that one call (ADR-012 §4.3).
  `from_spelling` SHALL map `invariant`, `pre` and `post` only in declaration
  head position; `pre(e)` in an expression stays `Expression::Pre`.
- The production SHALL read the kind from the leading token alone:
  `invariant` gives `Invariant`, `pre` gives `Precondition` and `post` gives
  `Postcondition`.
- The production SHALL carry the `at current` observation of an invariant as
  the kind itself. It adds no observation member, because the grammar admits
  no other observation for an invariant.
- The builder SHALL build `self` and `result` as leaf expressions and
  `reaches(a, b, edge)` as a node with two expression children and a member
  name. Nothing here checks where `self` or `result` may appear or what
  `edge` names; S3 does (FR-104).
- S2 SHALL NOT resolve the `using` alias, the model alias, the context type
  or the operation. It keeps their spellings and spans for S3.
- S2's forms depth limit SHALL apply to a state clause body exactly as it
  applies to a function body, with the same `StageFailure::Limit` refusal.
- The new `Expression` variants SHALL each have an owning family: `SelfRef`
  and `Result` belong to `ProtocolClause`, and `Reaches` to `StateModel`
  (ADR-012 §4.3, §15.2).

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-102-AC-1 | The unit `invariant ParentOrder using v on Config::ConfigVersion at current { present(self.parent) implies deref(value(self.parent)).versionNumber < self.versionNumber }` builds one `StateClauseForm` with kind `Invariant`, name `ParentOrder`, profile `v`, context `Config::ConfigVersion`, no operation, and a body whose spans resolve to the source bytes of each sub-expression. | Test (TC-456) |
| FR-102-AC-2 | `pre P using v on Config::ConfigVersion::attemptUpdate { true }` builds kind `Precondition` with operation `attemptUpdate`; the same text with `post` builds kind `Postcondition`. | Test (TC-456) |
| FR-102-AC-3 | `not reaches(self, self, parent)` builds `Not(Reaches { SelfRef, SelfRef, edge: parent })`, and `self.versionNumber = pre(self.versionNumber)` builds an equality of two `Field` reads of `SelfRef`, the right one under `Pre`. `result` builds `Expression::Result`. None refuses `UnrepresentedConstruct`. | Test (TC-456) |
| FR-102-AC-4 | The `dispatch` arms for `Invariant`, `Pre` and `Post` each make exactly one call (the existing `dispatch_entry_is_a_single_thin_call` check passes over them). A unit whose only declaration begins with a spelling no family claims, such as `temporal`, still refuses `NoDispatchEntry`, naming that spelling. | Test (TC-457) |
| FR-102-AC-5 | A state clause body nested one level deeper than `FormsLimits` allows refuses `StageFailure::Limit` with limit kind `nesting-depth-exceeded` at the body; at exactly the limit it builds. | Test (TC-457) |
| FR-102-AC-6 | With the `seam-probe` feature, the probe variant of `LeadingTokenKind` and of `Expression` still fails to compile at exactly the checked-in S2 seam list, which now includes the `protocol_clause` production entry (FR-063). | Test (TC-457) |

## Dependencies

- ADR-012 §15 (the mapping), §4.3 (thin seams), §5.1 S2.
- FR-067 and FR-091: the S2 builder and the `Value` forms this extends.
- The code change waits for the other lane's current work in `qsl-forms`
  (QSL-273 ticket text).
