---
id: FR-574
title: "Carry the FR-351 separating witness record on a decisive protocol counterexample"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-004
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/ADR-036
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-031
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-570
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-571
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-351
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-207
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-352
    type: depends_on
---
# FR-574: Carry the FR-351 separating witness record on a decisive protocol counterexample

## Description

Every `decisive-counterexample` result of FR-570 or FR-571 SHALL carry exactly
one QSpec FR-351 separating witness record, with the trace position of the
step at which the divergence or violation occurs, and its `value_path`
derived from QSpec FR-207 as below. It implements ADR-036 PA-24 (ADR-031
SW-5, SW-7 as amended there). quire-protocol FR-302 imports the record
byte-exactly and refuses a decisive realizability result without it.

## Inputs

A projection or realizability counterexample (FR-570, FR-571) that has
replayed (FR-573), and the request's assessed-subject root: the
observation identity and the FR-204 declared object identity the claim is
assessed over. The root is the claim's own binding of the subject it is
assessed over (QSpec FR-351 "the assessed subject's root"); the engine does
not choose, derive or default it.

## Outputs

One FR-351 record on the QSpec FR-352 `native-run-result/2` document. A
`closed-scope` result carries no record (quire-protocol FR-302).

## Behavior

The record has exactly these components:

| Component | Value |
| --- | --- |
| `deciding_quantifier` | Absent: the family has no quantifier occurrence. |
| `deciding_element` | The canonical string of the checked node id of the violated obligation (realizability) or of the divergent action's global node (projection). |
| `index` | Absent: the family assigns no positional index. |
| `value_path` | An FR-207 runtime value path, derived below. |
| `trace_position` | The decimal, zero-based index of the step at which the divergence or violation occurs in the counterexample's ordered event sequence: for indistinguishable histories the length of the longer of the two histories less one, for environment defeat the same index in the defeating play of the first owner branch (branches in canonical node order), and for a projection counterexample the index into `path_a` of the first divergent action, `path_a` being the canonically first of the two paths (FR-570). |

The `value_path` derivation:

- The canonical wire form shall be FR-207's: `kind: runtime_value`, a `root`
  with exactly `observation` and `subject`, and an ordered `steps` array.
- The engine shall copy `root.observation` and `root.subject` verbatim from
  the request's assessed-subject root: the observation identity, and the
  `object` subject whose declared object identity (FR-204) the request
  carries. The engine shall not invent, derive or default either.
- QSL shall refuse a request with the root or either component absent,
  `invalid_runtime_input`/`missing-member` naming it. The root is not a
  member of the FR-572 key and does not change the artifact identity. QSpec
  FR-290 defines no carrier for this binding, so the request input is stated
  here and the carrier is part of the open question on QSL-1898.
- `steps` shall be the empty sequence, which FR-207 defines as denoting the
  subject itself and never as an absent path. FR-207 has six step kinds and
  none selects a node of a checked protocol graph, so the derivation shall
  introduce no step and no rendered path: the node is identified by
  `deciding_element` and node references in the artifact remain checked
  node ids of the linked graph, which are not the trace position.
- A value path shall not be rendered text, a source span or a name search.

This derivation is a pin, not a QSpec change. A protocol-node step kind in
FR-207 would be an optional upstream improvement raised on QSL-1898; it does
not block this requirement and the requirement does not assume an answer.

The QSL record type (`SeparatingWitnessRecord`) is amended by ADR-036 PA-24
(an optional quantifier, a deciding element that is a kernel value or a
protocol element, and an absent `index` on the wire); a state-clause record
(FR-265) shall still round-trip unchanged.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-574-AC-1 | Every `refuted`/`decisive-counterexample` result of either kind carries exactly one record; a `proved`, `inconclusive`, `unsupported` or `failed` result carries none. | Test (TC-1414) |
| FR-574-AC-2 | The record has an absent `deciding_quantifier` and an absent `index`, and the wire form omits both; a record the strict reader sees with either present for these families, or with a missing `deciding_element`, `value_path` or `trace_position`, is refused (QSpec FR-351-AC-5). | Test (TC-1414) |
| FR-574-AC-3 | `deciding_element` equals the canonical string of the violated obligation's checked node id for a realizability counterexample, and of the divergent action's global node for a projection counterexample. | Test (TC-1414) |
| FR-574-AC-4 | For a projection counterexample whose divergent action is the third event of `path_a`, `trace_position` is `2`; for indistinguishable histories of lengths 3 and 5 it is `4`; for an environment defeat whose violation occurs at the fourth step of the first owner branch's defeating play it is `3`; two counterexamples that differ only in the step of divergence give different positions. | Test (TC-1414) |
| FR-574-AC-5 | `value_path` has the FR-207 canonical form with an `object` root subject and an empty `steps` array, compares componentwise, and is never absent; its root equals the request's assessed-subject root exactly; changing only display text, path or span leaves it equal. | Test (TC-1414) |
| FR-574-AC-7 | A request with the assessed-subject root absent, or its observation or object identity absent, is refused `invalid_runtime_input`/`missing-member` naming the absent component; a request differing only in the root yields an equal FR-572 key. | Test (TC-1414) |
| FR-574-AC-6 | A state-clause separating witness record (FR-265) with its quantifier and index round-trips unchanged through the amended record type and wire. | Test (TC-1414) |

## Dependencies

- ADR-036 PA-24; ADR-031 SW-5, SW-7.
- [FR-570](FR-570-project-a-global-protocol-onto-one-role.md), [FR-571](FR-571-decide-whether-local-strategies-realize-a-global-protocol.md).
- [FR-265](FR-265-derive-a-state-clause-separating-witness.md): the state-clause record that must not change.
- QSpec FR-351, FR-207, FR-352. Open on QSL-1898: an optional upstream improvement (a protocol-node step kind in FR-207) and the carrier of the assessed-subject binding; neither blocks this requirement.

## References

- ADR-036; QSL-1889, QSL-1893, QSL-1898.
