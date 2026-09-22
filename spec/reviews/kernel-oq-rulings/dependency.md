---
id: SR-512
title: "Kernel-convergence rulings OQ-A to OQ-F dependency review (PR #351)"
type: SpecReview
analysis: dependency
scope: "PR #351 diff: ADR-011 §6.1 layer-2 row, §6.1 kernel rules, §7.1 graph; ADR-013 T-6, C-26, QC-21 to QC-23, §8 OQ-A to OQ-F and ticket bullet; FR-084, FR-088, FR-091 Dependencies; TC-409, TC-410"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: reviews
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-084
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-088
    type: reviews
---
# SR-512: Kernel-convergence rulings OQ-A to OQ-F dependency review

## Summary

Checks the crate and layer edges the rulings add, the ownership of the work
they create, and the declared dependencies of the FRs they amend.

The OQ-A edge (layer 2 → K) is acyclic. K depends on nothing in the
ecosystem (ADR-011 `:603`). F → K already exists (`:604`). Layer 1 stays "F"
only. The §7.1 graph adds `QFM --> QX` (`:841`), which matches the §6.1 row
(`:606`). `src/forms/syntax.rs:13` already imports `quire_exact::{CollectionKind,
Integer}`, so the edge records current code and adds no new coupling. The
code work behind OQ-B to OQ-F is routed to QSL-131 consistently. The gaps are
enablement ones: one crate boundary is left undecided, and the new source
dependencies are not declared.

## Verdict

**ACCEPT WITH FINDINGS.** FND-001 (medium) leaves undecided which crate mints
compound-unit identities, and RT and CG depend on that answer. The rest are
low-severity bookkeeping.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Nothing decides which crate mints a compound `UnitId`. OQ-B makes a compound unit's `UnitId` the `quire.value.compound-unit/v1` digest, which FR-142 calls "an evaluator-owned value identity" produced by quantity multiplication, division and power (FR-142 `:134-142`). Its preimage is JCS over canonical-root unit node IDs, so computing it needs the unit graph and SHA-256/JCS. T-6 (`:848`) keeps "the preimage types, JCS and hashing" in QSL, and QC-15 says "QSL computes every digest". Today the kernel's quantity operations are add, subtract and compare only (`quire-exact/src/quantity.rs:65`, `:109`), and compound units are built in QSL `value::unit` (`src/value/unit.rs:64-74`). Quantity mul/div/pow therefore cannot be kernel operations, and RT and CG, which consume only the kernel (O-13 Owner row), have no way to produce a compound `UnitId`. **Fix:** state in T-6 or O-13 where quantity mul/div/pow and compound-unit minting live. Either QSL passes a precomputed compound `UnitId` into the kernel operation, from a unit table fixed at check time, or the kernel gains a hashing dependency, which would be an AD-016 kernel-row change needing a QC. Name the RT/CG consequence (TK-03). | ADR-013:848, :1026, :1093; QSpec FR-142:134-142; quire-exact/src/quantity.rs:65, :109; src/value/unit.rs:64-74 |
| FND-002 | low | QC-21, QC-22 and QC-23 have no filing ticket. The ticket bullet (`ADR-013:1200-1205`) files QC-1 to QC-20 as TK-06 to TK-10 and then says only that "QC-21 and QC-22 amend the AD-016 kernel row further, and QC-23 adds one FR-201 domain". Their "Blocks" cells say "nothing", which is correct, since QSL-131 follows the record asynchronously. But no QSpec ticket exists to land them, so AD-016 `:356` and FR-201 will keep the old text with nothing tracking the change. **Fix:** name the QSpec ticket (an existing TK or a new agent-ix/quire-specification issue) that carries QC-21 to QC-23, in each row or in the bullet. | ADR-013:1032-1034, :1200-1205 |
| FND-003 | low | The amended FRs do not declare their new dependencies. FR-084's Dependencies section does not list ADR-013 O-05/§8 OQ-C and OQ-E, quire-specification `model-complete.md` ("Object universe", "Reference identity key"), FR-204, or QSL-131, although the new section and AC-7 rest on all of them. FR-088's Dependencies (`ADR-013 §1 ... O-14, §4 C-26`) omits §8 OQ-D/OQ-F, QSpec FR-141 and FR-144, and QSL-131, which AC-11 depends on. **Fix:** add these to each FR's Dependencies section. | FR-084 Dependencies; FR-088 Dependencies |
| FND-004 | low | The rank change to C-26 has three owners. C-26's validation cell names "#213 S-3 test per type-node form" (`ADR-013:888`). FR-088 is QSL-158's (S-3b). FR-088-AC-11, TC-409 and the O-14 remaining work are QSL-131's. Nothing says which ticket changes C-26's conversion to emit ranks and the FR-141 `VariantId`, or whether QSL-131 extends a QSL-158 FR in place. **Fix:** in O-14's "Remaining work" and FR-088's Status, name QSL-131 as the ticket that changes C-26 (conversion and TC-252/TC-409), and state that FR-088 is amended in place for it. | ADR-013:360, :888; FR-088 Status; spec/spec.md:468 |
