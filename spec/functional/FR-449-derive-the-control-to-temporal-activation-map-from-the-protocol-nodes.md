---
id: FR-449
title: "Derive the control-to-temporal activation map from the protocol nodes"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-024
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-218
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-337
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-381
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-383
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-384
    type: references
  - target: ix://agent-ix/quire-specification/FR-464
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-370
    type: references
  - target: ix://agent-ix/quire-specification/FR-479
    type: references
---
# FR-449: Derive the control-to-temporal activation map from the protocol nodes

## Description

QSL SHALL derive, from the protocol nodes of a checked package (FR-383), the
control-to-temporal activation map of each protocol: which activation of the
protocol requires which temporal clause. The map is QSpec FR-464's
"Activation relation". The derivation reads the `protocol_clause` node and
the nodes it references only, so the map derived from the in-process checked
package equals the map derived from the package read back through QSL's I2
reader. No node, artifact or document carries the map: S4 writes no
activation-map wire and no compiled-protocol document (FR-383), and the map
is a function of the package.

## Use case

A protocol conformance assessor receives a package for an order protocol
that is activated by each `OrderStarted` event and requires the temporal
clause `OrderClosureBounded`. Before any workflow runs it needs to know,
for each activation of the protocol, which temporal clauses that activation
must satisfy, so that each obligation it later evaluates is the one the
source authored and not one inferred from a matching clock or a table
position. It reads the map from the package and from nothing else.

## Semantic authority and boundary

QSpec FR-464 owns the clause body (`activation`, `requirements`), the
derivation rule and the reader's refusals. FR-218 owns the binding of each
`requires temporal` member to the temporal clause it names, FR-381 the
protocol's activation, and FR-337 the temporal clause node. This
requirement specifies what QSL derives and returns. The map selects no
truth: each required temporal clause keeps its own body, activation and
evaluation (QSpec FR-370; FR-327 to FR-329). An obligation is bound to a row
only by QSpec FR-479's protocol-obligation activation binding, which the
protocol consumer (QProtocol) constructs from the row and its admitted
trigger observation; QSL derives the rows and constructs no binding.

## Inputs

- The `protocol_clause` node of one protocol (FR-383) and the nodes its
  `activation` and `requirements` arguments reference: from the in-process
  `CheckedPackage` or from QSL's I2 read.

## Outputs

- The protocol's `ActivationMap`: an ordered list of rows, one per temporal
  clause node in the clause's `requirements` set, in that set's order
  (ascending node-id digest). A row names the `protocol_clause` node key,
  the activation (`Origin`, or `Each` with the key of the trigger's
  `value`/`parameter` node) and the temporal clause node key.
- Or, for a package the I2 reader refuses, that refusal (QSpec FR-464), and
  no map.

## Behavior

- QSL SHALL derive a protocol's map from its `protocol_clause` node's
  `activation` and `requirements` arguments only. It SHALL NOT read source
  text, a name, a control or temporal clause position, a matching profile or
  clock, a timestamp or an observation to add, remove or substitute a row.
- A protocol whose `requires temporal` members name no temporal clause has
  an empty map. A temporal clause named by two `requires temporal` members is
  one `requirements` entry (FR-218) and so one row.
- A temporal clause of the unit that the protocol does not require has no row
  in that protocol's map, whatever clock, profile or name it shares with the
  protocol.
- The map of a protocol SHALL be equal for the in-process package and for
  the package read back through the I2 reader, and equal for two compiles of
  the same unit; a source change that leaves the `protocol_clause` node key
  unchanged leaves the map unchanged.
- If a read-back package holds a `requirements` entry or an `each` trigger
  whose target is absent from the clause's `dependencies`, then QSL's I2
  reader SHALL refuse the package with `missing_declaration`/`missing-name`
  at the target (QSpec FR-464-AC-6), and QSL SHALL return no map, partial or
  whole.
- A `requires temporal` name that resolves to no temporal clause of the unit
  is refused at S3 (FR-218), so S4 emits no protocol node and no map exists
  for that unit.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-449-AC-1 | Over fixture **W** (TC-1061: `activation on each (opened: M::OrderStarted)`, `requires temporal OrderClosureBounded`), the map has exactly one row: the `OrderFlow` clause key, `Each` with `opened`'s node key, and `OrderClosureBounded`'s node key. | Test (TC-1194) |
| FR-449-AC-2 | **W** with `activation on origin` gives one row carrying `Origin` and no trigger; **W** with no `requires temporal` gives an empty map; **W** with a second `requires temporal OrderClosureBounded` gives AC-1's map. | Test (TC-1194) |
| FR-449-AC-3 | **W** with a second temporal clause `OrderPaidBounded` and `requires temporal OrderPaidBounded` gives two rows in ascending node-id digest order of the two temporal clause nodes; **W** with a temporal clause `RefundBounded` on the clock `refund-clock` that the protocol does not require gives AC-1's map. | Test (TC-1194) |
| FR-449-AC-4 | The map of **W** derived from the in-process package equals the map derived from the package read back through the I2 reader; compiling **W** twice, and compiling **W** reformatted with a comment added, give equal maps. | Test (TC-1194) |
| FR-449-AC-5 | **W**'s emitted package edited so that the `requirements` entry, or separately the `each` trigger, names a node absent from the clause's `dependencies` is refused by the I2 reader with `missing_declaration`/`missing-name` at the target, and no map is returned. | Test (TC-1194) |
| FR-449-AC-6 | **W** with `requires temporal Missing`, where the unit declares no temporal clause `Missing`, is refused at S3 with `missing_declaration`/`missing-name` at the name, with no package and no map. | Test (TC-1194) |

## Dependencies

- [FR-383](FR-383-emit-a-checked-protocol-as-v2-protocol-nodes.md) (the
  `protocol_clause` node and its keys).
- [FR-218](FR-218-check-every-protocol-control-construct-at-s3.md) (the
  `requires temporal` binding) and
  [FR-381](FR-381-check-protocol-captures-activation-and-compensation-recovery-at-s3.md)
  (the activation).
- [FR-337](FR-337-emit-a-checked-temporal-clause-as-a-v2-temporal-clause-node.md)
  (the temporal clause node).
- QSpec FR-464 "Activation relation".
