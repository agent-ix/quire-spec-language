---
id: FR-320
title: "Lower and emit union, construction and case nodes on the v2 wire"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-032
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-012
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-093
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-095
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-319
    type: traces_to
  - target: ix://agent-ix/quire-specification/FR-322
    type: depends_on
---
# FR-320: Lower and emit union, construction and case nodes on the v2 wire

## Description

When S3 lowers and S4 emits a checked package that contains a union, a union
construction or a `case`, the compiler SHALL spell each as QSpec's v2 union
type node, `union_value` node and `case` node (QSpec FR-440), and SHALL produce a package whose identities survive
the I2 read and replay's recompile (ADR-012 §16.3 SC-R1, §16.4 S3 lowering
and S4 rows).

## Inputs

- Checked union, construction and `case` nodes with their FR-319 keys.

## Outputs

- v2 nodes and members in QSpec's union spelling, in the emitted checked
  package; occurrences in the v2 source map.

## Behavior

- S3 lowering SHALL map the union type node, a construction and a `case` to
  v2 nodes through the wire vocabulary (`NodeTag`, `SemanticTerm`,
  `Operator`), each with QSpec FR-440's spelling. A `case` node's
  `quire.op.control.case` application carries `member: null`.
- The S4 emitter SHALL omit with `UnsupportedForm` every node whose tag and
  form the target IR cannot decode, together with every node that names it,
  leaving no partial body.
- S3 lowering SHALL record the occurrences of every `case` and construction
  node.
- When a replayed evaluation of a function body that contains `case` fails,
  the replay facade SHALL report the failing node's occurrence key through
  the v2 source map (FR-095).
- For a `value-validity` item whose function body contains `case`, the
  downstream IR and CG arms settle the item `unsupported` with a catalog code
  when no backend proves it; S4 SHALL emit the item, leaving its settlement
  to those arms.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-320-AC-1 | A package with `Shape`, `area` (FR-318-AC-1) and a function returning `Shape::Rect(2, 3)` emits v2 nodes in QSpec's union spelling. After S4 emission and the I2 read, the union's node id and every member identity are unchanged (the `WireNodeId` equals the emitted id), and after replay's S1 to S4 recompile of the digest-addressed source the recompiled `package_id` equals the emitted one. | Test (TC-827) |
| FR-320-AC-2 | Against an IR that cannot decode the union (tag, form), S4 omits the union node and every node naming it with `UnsupportedForm`, and the emitted body holds none of them. | Test (TC-827) |
| FR-320-AC-3 | One `value-validity` item whose function body contains `case` is emitted and settles `unsupported` with its catalog code through the IR and CG arms, with no QSL settlement in between. | Test (TC-828) |

## Dependencies

- QSpec FR-322 (checked package artifact) and the v2 schema.
- FR-093 (lowering), FR-095 (source map), FR-319 (keys).

## References

- Owning ticket: QSL-383. Design: ADR-012 §16.3, §16.4, §16.10.
- v2 union node spelling (SC-G1, SC-G2): QSpec FR-440 (specification
  ticket STD-142).
