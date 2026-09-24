---
id: SR-608
title: "Failure-domain review of the QSL-210 model-owned, StateModel and quantity node-key requirements"
type: SpecReview
analysis: failure-domain
scope: "Commit ac4f974f: FR-094 identity model (ModelOwner, EffectiveId and DeclarationKey, declared and compound UnitId), collisions, recursion, unmapped inputs and key determinism"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-094
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-092
    type: reviews
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: reviews
  - target: ix://agent-ix/quire-specification/FR-322
    type: references
---

## Summary

These parts of the design hold:

- **Domain packages.** Two domain packages that use the same IR node string
  key apart, because `ModelOwner.identity` differs.
- **Versions.** Two versions of one package key apart (M1 and M2, R1 and R2,
  C1 and C3).
- **Equal clause text.** Equal clause text on two members keys apart (C1 and
  C4).
- **Precondition and body.** One member's precondition and body key apart,
  through the `clause` binding.
- **Recursion.** An empty model body keeps mutually referencing object types
  out of recursion groups. A clause body that dispatches back to its own
  operation references the model node, not a clause function, so it forms
  no recursion group.
- **EffectiveId.** No preimage reads an `EffectiveId`, so a change to a
  normalization rule moves no key.
- **Labels.** Clause-function labels stay out of the preimage.
- **Units.** Declared and compound `UnitId`s cannot be confused: `u` and
  `u^1` key apart (U4).
- **Names.** Model nodes carry no `declaration`, so they never meet the
  reader's `ambiguous-name` or `declaration-nominal-mismatch` checks.

Four failures remain:

- One model declaration node id can stand for two different declarations.
- Clause functions are built twice for one package, with no rule for the
  duplicates.
- A population type loses `T` in the checked type.
- A clause function cannot yet be traced back to its operation.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | One model declaration node id can name two different declarations. A model node's preimage is `ModelOwner{identity, version, node}` over an empty body. QSpec FR-321 states that two selections with equal identity and version and different digests are unequal. So two domain packages `acme/orders` `1.0.0` with different bytes give the same M1, R1, S1, PO1 and P5 keys to different `Order` declarations, for example with different attribute types. That breaks O-04's "equal ids mean structurally identical nodes". A consumer that joins two artifacts by node id then merges the two declarations. QSpec's own `ModelOwner` enum and dimension vectors avoid this because their preimages carry the declaration's content. FR-094's justification is that "the lock selects the domain package's bytes by digest". That holds only within one lock. Fix: add a body binding that pins the content, either the selection's digest or a digest of the declaration's record. Alternatively, record in QC-25 that an identity and version must name one digest, and state where that is refused. Add a vector pair that differs only in that binding. | FR-094:97-107; QSpec FR-321:30 (`e72756f`); ADR-013 O-04 Equality; QSpec `node-identity-vectors.json` `model-enum-order-status` |
| FND-002 | medium | Clause functions are built more than once for one package. A redefiner `B.size` is a candidate in root `A.size`'s dispatch table and in its own `checked_dispatch_operation` call. So its authored precondition, its effective combinator and its body are each built twice, with identical preimages. FR-092 says `check` SHALL refuse "a package in which two distinct nodes have equal preimages and are not the same node" (`unsupported-feature`). Neither FR says whether these duplicates are the same node, so a valid model may refuse. The same holds for an authored precondition that several roots reach. Fix: state that equal-preimage clause functions, and equal-preimage nodes outside recursion groups in general, are one node, merged by key. Limit FR-092's refusal to recursion-group nodes. Add an AC with a redefiner that shows one node per clause. | FR-092:122-133; FR-094:166-170; `qsl-semantics/src/check/checked_dispatch.rs:975-1020` |
| FND-003 | medium | A population type loses its object type in the checked type. `ValueType::Population(u64)` carries only `N`, so `p: Population<M::Order>[3]` and `q: Population<M::Invoice>[3]` have equal checked types. The nodes differ only through `semantic_type` (S1 over R1 over M1). A type-node cache keyed by `ValueType`, which is how FR-092's other types are built, gives both the PO1 key. No AC or vector covers two populations with the same `N` and different `T`. Status leaves the choice of carrier to A4b. Fix: add a vector for `Population<M::Invoice>[3]` that differs from PO1, and an AC that `p` and `q` get different parameter nodes. Require that a population type node is built from the binder's resolved form or from a checked type that carries `T`, and never looked up by `ValueType` alone. | FR-094:59-61, :151, :555-557; `quire-exact/src/value.rs:204-205` |
| FND-004 | low | A reader can find a clause function's operation only through its preimage `owner`, which no v2 member carries. QC-25 already asks how a reader recovers `ModelOwner.node`. In the meantime, nothing in the graph links a `dispatch_call` node, which names M1 and `size`, to the clause functions C1 and C2. A reader cannot tell that C1 is `Order.size`'s precondition without recomputing preimages from the domain package. Fix: add to QC-25 how a dispatch site reaches its candidates' clause functions, either as correspondence entries for clause functions or as an edge from the model node. Until then, state that the link is recovered only by recompiling. | FR-094:137-142, :186-190; ADR-013 QC-25 |

## Resolution

All findings are fixed. FND-001: FR-094 states that within one check intake admits one selection per identity, and that across checks one (`identity`, `version`) keys the same node for any digest, as QSpec's `ModelOwner` nominal nodes do; ADR-013 O-04 keeps digests out of the preimage, so QC-25 asks QSpec to make one (`identity`, `version`) name one digest. FND-002: FR-092's equal-preimage refusal is limited to in-group nodes of different recursion groups, equal-preimage nodes outside groups are one node, and FR-094-AC-5 and TC-418 step 4 build `Sub.size`'s clauses from both dispatch operations. FND-003: FR-094 builds the `Population` node from the resolved form, and vector PO3 (`Population<Invoice>[3]`, over R5 and S3) differs from PO1 (AC-3, TC-417 step 5). FND-004: FR-094 states that the dispatch tables carry the link, and QC-25 asks QSpec for its v2 spelling.
