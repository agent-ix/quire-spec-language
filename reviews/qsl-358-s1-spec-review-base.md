---
id: SR-954
title: "QSL-358 slice 1 spec review of PR 565: ADR-011 layer SV, FB-05 shared-leaf class, T-12/T12-B design, X-11, and the FR/TC edits"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@a7df1ff07f56e2b37249841138600c86c1b3d1b5; spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md; spec/functional/FR-059, FR-060, FR-061, FR-068; spec/test-cases/TC-156, TC-157, TC-158, TC-175, TC-390; spec/tests.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-059
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-390
    type: reviews
---
## Summary

Ticket: QSL-358 (slice 1). PR: quire-spec-language#565 at a7df1ff0. Base
checklist over the spec diff, checked against the crate as built.

Clean:

- Status line records the owner's FB-05 widening and names X-11.
- Decision 6 and "The approved crate extractions are X-1 to X-11" agree with the
  X-11 row.
- §6.1 layer rows 3, 4, 5, R and 6 add SV to "Depends on". That cell is a
  permission, so adding SV to R and 6, which do not use it yet, is allowed.
- FB-05 says a crate joins the class "only by an amendment of this row", which
  keeps the class closed.
- The T-12 duplicate-revision clause matches FR-061 and the code: both leaves
  still classify as QSL there.
- FR-059's statement, FR-060's scan-tree sentence, FR-061's leaf paragraph and
  FR-068's import list match the code. The new ACs (FR-059-AC-9, FR-060-AC-5)
  are atomic and testable, and TC-156/157/158 have matching steps and expected
  results. `spec/tests.md` ranges are updated.

T12-B design, judged on its own terms: a separate `decode_admitted` constructor
with no preimage input and its own reader-only allow-list does keep `check` the
only caller of `from_digest`, and stating "the guarantee is the call-site
allow-list plus the lookup discipline, not the type" is honest. The hole is in
the lookup discipline (FND-003).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FB-05's row says SV "depends on K only", and FR-059's statement says the same. The SV bullet, the §6.1 SV row and X-11 in the same ADR list `quire-canonical`, `serde` and `thiserror`, and the crate depends on all three. `quire-canonical` is an agent-ix crate, so this is the governing row understating an ecosystem edge. Say it depends on K and ADR-013 §2's one RFC 8785 encoder, and on no QSL layer. The no-cycle argument still holds: `quire-canonical` depends on no QSL, IR, RT or CG crate. | spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md:572; spec/functional/FR-059-check-backend-dependency-direction.md:46-50 |
| FND-002 | medium | The ADR does not record SV's `semantic_node` module. The crate exposes `quire_semantic_value::semantic_node` (`InvalidSemanticGraph`, `SemanticGraphCause`, `check_terms`, `IDENTITY_LIMITS`), and every caller imports the refusal vocabulary from there. The §6.1 SV row lists only `stop`, `quantity` and `unit`'s runtime half. X-11 says the public items are at `quire_semantic_value::{stop, quantity, unit}`. The §6.2 move row names only `stop`, `quantity` and `unit`. Only the SV bullet mentions `IDENTITY_LIMITS`. Add `semantic_node` (the refusal vocabulary, the term check and the identity limits) to all three rows. | spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md:715,900,1099 |
| FND-003 | medium | The T12-B design's lookup discipline is circular in a backend. It says a decoded key "resolves by lookup among the admitted package's node table", so "a fabricated digest resolves to no node". But in RT the node table's own keys also come from `decode_admitted` on the same package bytes. A forged package therefore yields forged keys that resolve. The guarantee only holds if the package's key bytes are bound by an identity check that ran before decoding (the I2 reader's verification, or IR's admission binding `package_id` to the bytes). The design does not state that precondition. It also does not say that a decoded key, which has the same type as a minted one, must not feed constructors that assume `check` provenance, such as SV's `UnitGraph::from_checked_nodes` / `declared_unit_id`. State both, and name the identity check the allow-listed reader module must run first. | spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md:1440 (T-12, "Decoding an admitted key") |
| FND-004 | low | The SV bullet says "SV mints no kernel identity", then "The one digest it computes is a compound unit's ... `UnitId`". `UnitId` is a kernel identity type (§6.1 K row), so the two sentences contradict. Say SV mints no `NodeKey`, `EffectiveId` or `PopulationId`, and computes one kernel identity, the compound `UnitId`. | spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md:780-797 |
| FND-005 | low | Text that looks ahead to later slices. X-11 says "**Extracted** in part" without saying what the rest is, which implies more moves (slice 2's `containment`) without recording them. The edge row says "RT and CG take the edge when they adopt the crate". T-12 ends with "Follow-up: add `NodeKey::decode_admitted` ... then repoint RT's `from_bytes` callers when RT adopts the kernel `NodeKey`". An ADR states the decision; sequencing belongs in the ticket. Say "Extracted", list what is in the crate, and drop the follow-up and "when they adopt" clauses. T-12 can say the carve-out is designed and not built. | spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md:987,1099,1440 |
| FND-006 | low | §7.1 does not match the built graph. The mermaid draws `QSE & QEV --> SV` but leaves out `QPK --> SV`, although `qsl-package` has a normal dependency on SV (for `IDENTITY_LIMITS`). The SV bullet says layers "3, 4, 5, R and 6 import its items", but `qsl-route` and `qsl-replay` do not depend on SV. Add `QPK` to the edge, and phrase the bullet as permission ("may import") or list only the crates that do. | spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md:784-786,959 |
| FND-007 | low | TC-390 step 2 now adds `quire-semantic-value/src/`, but its `semantic_value` list still reads `src/value/{definition, enumeration, unit, quantity, key, reference}`. `quantity` moved to SV in this PR, `key` and `reference` are long gone, and those modules live under `qsl-semantics/src/value/`, not `src/value/`. Make the list match the test's `BELOW_CORE` constant. | spec/test-cases/TC-390-family-outcome-and-refusal-layering.md:44-47 |

## Verdict

The amendments are sound in structure: the FB-05 class is closed and named, the
direction check and the duplicate-revision check treat the leaves correctly, and
the FR/TC edits match the code. FND-001 and FND-002 are accuracy defects in
governing rows and should be fixed in this PR. FND-003 is a gap in the T12-B
design, not in shipped code; fix the design text now, before RT builds against it.
The lows are quick wording fixes.
