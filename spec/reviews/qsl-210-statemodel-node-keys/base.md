---
id: SR-606
title: "Base review of the QSL-210 model-owned, StateModel and quantity node-key requirements"
type: SpecReview
analysis: base
scope: "Commit ac4f974f: FR-094, TC-417 to TC-419, and the FR-092, FR-093, ADR-013 (O-04, QC-3, QC-25, QC-26), spec.md, tests.md and US-005 edits"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-094
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-092
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-093
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-417
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-418
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-419
    type: reviews
  - target: ix://agent-ix/quire-specification/FR-322
    type: references
---

## Summary

The IDs are well formed and unused before this commit: FR-094, TC-417 to
TC-419, FR-094-AC-1 to AC-7 and FR-094-CON-1 and CON-2. `quire validate`
adds no warning for the changed files, and `make check-index-completeness`
passes. Every AC and CON has a TC. The `tests.md` rows list the same ACs as
each TC's Scope line; CONs are left out of the rows, as in the existing
TC-413 to TC-416 rows. US-005 and `spec.md` index FR-094.

The golden vectors were recomputed with a scratch script. All 28 blocks (M1
to M4, R1 to R3, S1, S2, PO1, PO2, P5 to P7, E4 to E9, C1 to C4, U1 to U4)
equal their own RFC 8785 re-serialization. Each hashes with SHA-256 to its
`Key:` line and to its summary-table row. Every cited digest is the key of
the vector it names:

- R1 and R2 cite M1 and M2. R3 and S1 cite R1. S2, PO1 and PO2 cite S1 and
  FR-092's T2.
- P5 cites PO1. P6 and P7 cite R1. All three also cite T2 and T3.
- E4 cites P5, M1 and S2. E5 cites P5, P6, M1 and R1. E6 cites the same with
  R3. E7 cites P6 and M1. E8 cites E7, M1 and FR-092's T4. E9 cites P6, M1
  and T2.
- C1, C3 and C4 cite P7, FR-092's L1, T1 and T3. C2 cites P7, FR-092's L2,
  T2 and T3.
- U1, U2 and U4 cite QSpec's `unit-metre` key (`79637623…`), and U2 also
  `unit-second` (`d0f0c5d2…`). Both match `node-identity-vectors.json` at
  `e72756f`.

FR-092's 30 vectors still recompute.

The gaps are in the clause-function criteria. One AC cannot pass as written,
and part of the owner rule has no vector.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-094-AC-5 says "no preimage contains the synthesized function name", and this cannot pass for C2. `checked_dispatch_operation` names a body clause function `candidate.node`, so the body of `Order.size` is named `ix://acme/orders/Order/size`. That string is C2's `owner.node`. TC-418 step 4 searches only for `.precondition`, so it does not test the body case. Fix: restate AC-5 as label independence. Build the same clause functions under a different synthesized name and assert equal keys. Replace TC-418 step 4 with that rebuild. | FR-094:182-184, :517; TC-418 step 4; `qsl-semantics/src/check/checked_dispatch.rs:956-958` |
| FND-002 | medium | Two parts of the clause-function owner rule have no vector or AC. First, an authored precondition's owner is its author, even when the dispatch candidate inherits it. Second, the effective precondition's owner is the candidate. The Behavior text is also wrong about what the code builds. It says `checked_dispatch` builds three functions per dispatch candidate. The code builds one function per authored precondition, shared by every candidate that reaches it. It builds an effective `or` combinator only when two or more terms contribute; with one term it reuses the authored function. AC-5 uses one member with no ancestors, so neither part of the rule runs. Fix: describe the three cases as the code builds them. Add a `Sub.size` that redefines `Order.size` with its own precondition. Add vectors for Sub's authored precondition (owner `Sub/size`) and its effective combinator (owner `Sub/size`, body `or`). Assert that `Order.size`'s authored function keeps the `Order/size` owner in Sub's closure. | FR-094:166-180, :517; `checked_dispatch.rs:869-936` |
| FND-003 | low | The Refusals section lists "a `DeclarationKey` … whose `node` is empty". AC-7 and TC-417 step 7 test only the unknown `package` case. Fix: add the empty-`node` case to AC-7 and step 7. | FR-094:221-222, :519; TC-417 step 7 |
| FND-004 | low | TC-419 step 2 builds `second^-1 metre^1` "terms supplied in that order". `UnitGraph::compound_unit` refuses unsorted terms (`CompoundUnitCause::UnsortedTerms`), so no constructor takes that order. The step therefore cannot show that terms are sorted before keying. Fix: form U1 and U2 from checked expressions (`a * a` and `a / t` over `metre` and `second` parameters). This also covers the stage-formed units in SR-607 FND-001. | TC-419 step 2; `qsl-semantics/src/value/unit.rs` `compound_unit` (UnsortedTerms) |

## Resolution

All findings are fixed. FND-001: FR-094-AC-5 now requires that renaming the synthesized label leaves the keys unchanged, the text says the label appears only where it coincides with the owner's `node`, and TC-418 step 5 rebuilds under other labels. FND-002: the Clause function nodes section states what `checked_dispatch` builds (one authored precondition per member, an effective function only for two or more terms, one body per candidate) and each one's owner; vectors M5, R4, P8, L4, E10, C5 and C6 cover a redefining `Sub.size` with its own precondition, and AC-5 and TC-418 step 4 assert them. FND-003: the empty-`node` refusal is in FR-094-AC-7 and TC-417 step 7. FND-004: TC-419 step 2 and FR-094-AC-6 form U1 to U4 from checked `a * a`, `a / t`, `a / a` and `a * a / a`.
