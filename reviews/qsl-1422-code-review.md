---
id: SR-1988007
title: "Code review — QSL-1422 owner-model refusal pointer"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@a159eb94ca7f0207ec9aecf8112752faa2177f91; base b33520e2b5588c3df71f0bd11416e6af7fba6780; qsl-package/src/emit/tests/owner_model.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-383
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-1063
    type: reviews
---

## Summary

Ticket QSL-1422; reviewed the one-line expected pointer change in PR #2002, with the Rust test lane. The new `/owner_model/node` locus matches the current reader's refusal for the external `{package, node}` key fixture; no production reader or emitter behavior changed.

## Verdict

**PASS** — the exact malformed-wire refusal remains asserted at the field the current reader rejects, and the focused test passes.

## Examined Scope

- `FR-383-AC-27` (`spec/functional/FR-383-emit-a-checked-protocol-as-v2-protocol-nodes.md`): “Over the emitted package, a clause without the edge, an edge on another node, an external edge and an edge omitted from `dependencies` each refuse as IR's reader does (`malformed_wire`, `malformed_wire`, `malformed_wire`, `invalid_semantic_graph)”`.
- `FR-464-AC-16` (`../qspec-std619/spec/objects/interfaces/FR-464-checked-package-protocol-node-bodies.md`): “Every `protocol_clause` node has one `owner_model` local `NodeRef` to its exact model declaration node; the edge is present in both V2 schemas, included in the clause's exact dependencies, and absent from every other node. A missing edge, external key, non-model target or missing dependency refuses at the edge or dependency before clause interpretation, in the stage and with the code that FR-464-AC-20 states.”
- `FR-464-AC-20` (same QSpec file): “an omitted edge and an external key refuse `malformed_wire` at the node”.
- `TC-1063` row 11 (`spec/test-cases/TC-1063-protocol-emission-refusals-and-read-back.md`): “a clause with its `owner_model` removed, copied to a `role`, written as an external key, left out of `dependencies`, pointed at a Boolean scalar node | the reader refuses the rest `malformed_wire`, `malformed_wire`, `malformed_wire`, `invalid_semantic_graph`, and `ill_typed`/`operator-ineligible`”.
- `qsl-package/src/emit/tests/owner_model.rs`, `each_defect_of_the_edge_is_refused_where_the_reader_reaches_it`: reviewed the `external` fixture, its expected refusal, and the `FR-383-AC-26`/`TC-1063` trace binding.

The fixture is the published external-key shape `{ "package": "other", "node": owner }`. The current IR `CheckedNodeId` wire representation is flat (`package`, `path`, `ordinal`); its closed deserializer rejects the unexpected nested `node` field and serde-path reports that field as the precise locus. The test still checks `MalformedWire`, now at that nested field. The test's `assert_ne!(found, "admitted")` and exact expected string remain effective oracles.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |
