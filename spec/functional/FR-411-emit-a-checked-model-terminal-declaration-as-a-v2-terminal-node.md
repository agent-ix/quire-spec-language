---
id: FR-411
title: "Emit a checked model terminal declaration as a v2 terminal node"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-015
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-018
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-124
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-093
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-128
    type: depends_on
---
# FR-411: Emit a checked model terminal declaration as a v2 terminal node

## Description

The S4 package emitter (ADR-011 layer 4 `package`, edge E4) SHALL emit each
checked model `terminal` declaration (FR-124) as one
`quire.checked-semantic-graph/v2` node with `node_tag: "temporal"` and
`semantic_form: "terminal"`, whose body is the aggregate QSpec FR-476 states,
and QSL's I2 reader SHALL read that node back. QSpec FR-476 owns the body,
the declaration path, the identity rules and the reader's refusals; this
requirement states what QSL's emitter writes from its checked
`TerminalDeclaration` and what its I2 reader returns.

## Use case

A verification operator writes `terminal Done using v on Counter when {
self.value = 3 };` for a counter that stops at 3 and runs its deadlock-freedom
item. The package carries the declaration, so the model checker, the deadlock
classifier and replay of a deadlock counterexample all read the same intended
terminal states from the recompiled package, and the item's obligation
identity changes when the declaration does.

## Inputs

- A checked state model's `TerminalDeclaration::{When(clause), Any}` (FR-124),
  with its model alias and the span of its declaration.
- The node keys FR-092 mints and FR-093's lowering of a Boolean expression.

## Outputs

- One `temporal`/`terminal` node per checked declaration, body as QSpec
  FR-476 states, with `semantic_type` the package's `scalar_type`/`boolean`
  node, no `declaration` (QSpec FR-476: the node is content-addressed and
  nested under its model by its `model` reference), and one occurrence of
  role `declaration` at the declaration's span.
- For `When(clause)`: the clause's `expression` node, its body the predicate's
  Boolean term as FR-093 lowers it, and the `self` parameter node when the
  predicate reads `self`.
- No node for `TerminalDeclaration::None`.
- From the I2 reader: the model's `TerminalDeclaration`, or FR-476's refusal
  with its locus.

## Behavior

- The emitter SHALL write `model` as a reference to the `model` node of the
  declaration's model alias.
- The emitter SHALL write `mode` as `when` for `When(clause)` and `any` for
  `Any`.
- The emitter SHALL write `parameters` as an aggregate holding one
  reference, to the `self` parameter node at level 0 typed `Reference<M::T>`
  as FR-124 types it, when the predicate reads `self`, and as the empty
  aggregate otherwise and for `Any`.
- The emitter SHALL write `condition` as a reference to the predicate's
  `expression` node for `When(clause)`, and as the empty aggregate for `Any`.
- The emitter SHALL key the node by FR-322's rules over its body, so the mode, the model and the predicate enter the node id
  and the `package_id`, and the written name `T` enters neither.
- S4 SHALL NOT refuse a checked `terminal` declaration as not emitted.
- The terminal node enters the package's `package_id`, and a model
  subject binds its package through that `package_id` (QSpec FR-361,
  FR-476 "Identity"), so every obligation identity over the subject, the
  derived deadlock-freedom item's included, changes with the declaration.
  The request carries no separate terminal member.
- Replay of a deadlock-freedom counterexample (FR-128) SHALL read the
  model's `TerminalDeclaration` from the recompiled package's
  `temporal`/`terminal` node.
- The I2 reader SHALL admit an emitted terminal node and return the model's
  `TerminalDeclaration`, and return `TerminalDeclaration::None` for a model
  with no terminal node.
- The I2 reader SHALL refuse a node that breaks an FR-476 rule with FR-476's
  code at FR-476's locus, in FR-476's reader order.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-411-AC-1 | Compiling FR-126-AC-9's `Counter` unit with `terminal Done using v on Counter when { self.value = 3 };` emits one `temporal`/`terminal` node with `mode` `when`, `parameters` an aggregate holding one reference to the `self` parameter node, `condition` a reference to the `expression` node of `self.value = 3`, no `declaration` and one `declaration` occurrence at the declaration's span; with `terminal Halt using v on Counter any;` it emits `mode` `any` with empty `parameters` and `condition`; without a declaration it emits no terminal node. The I2 reader returns `When`, `Any` and `None` respectively. No checked declaration is refused as not emitted. | Test (TC-1142) |
| FR-411-AC-2 | AC-1's `when` and `any` units have different `package_id`s; renaming `Done` to `Stop` keeps the node id and `package_id`; the deadlock-freedom item over `Counter` with the `when` declaration and with none have different `package_id`s and so different obligation identities, with no other request member differing. | Test (TC-1142) |
| FR-411-AC-3 | FR-128-AC-6's unit, with its declaration written `terminal Done using v on Counter when { 6 / (3 - self.value) = 0 };`, compiles to a package holding its terminal node, and replay of its deadlock-freedom counterexample settles as FR-128-AC-6 states. The same counterexample in a request for the unit with `terminal Halt using v on Counter any;` in its place settles `inconclusive`, `Verdicts`: the recompiled package's terminal node marks `value = 3` intended, so replay finds neither a deadlock nor an undefined predicate there. | Test (TC-1142) |
| FR-411-AC-4 | Fed to QSL's I2 reader, with `dependencies` set to the body's targets, the node key, projection entry and `package_id` recomputed after each edit, AC-1's `when` node with `mode` `some` refuses `invalid_package`/`invalid-value`, and with `condition` the empty aggregate refuses `invalid_semantic_graph`, each at FR-476's locus. | Test (TC-1142) |

## Dependencies

- ADR-011 §6.1 layer 4 `package` and §2.1 E4; ADR-018 DL-1 to DL-3.
- [FR-124](FR-124-declare-intended-terminal-states-and-derive-deadlock-freedom.md)
  (the checked declaration and the derived item),
  [FR-382](FR-382-build-the-terminal-declaration-form-at-s2.md) (the S2
  form), [FR-093](FR-093-lower-checked-value-expressions-to-fr-322-terms.md)
  (lowering the predicate), FR-092 (node keys),
  [FR-128](FR-128-replay-a-model-counterexample.md) (deadlock counterexample
  replay).

## References

- QSpec FR-476 (checked-package model terminal declaration body), FR-366
  (the declaration and deadlock freedom), FR-322.
- Owning ticket: Linear STD-303.
