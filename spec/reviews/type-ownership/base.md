---
id: SR-482
title: "Base checklist review of ADR-013 canonical type, package and conversion ownership"
type: SpecReview
analysis: base
scope: "spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md, spec/spec.md"
review_set: all
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: reviews
---
# SR-482: Base checklist review of ADR-013

## Summary

Reviewed commit 660aa25 on `task/211-type-ownership` against the quoin
spec-review checklist. ADR-013 is a design decision record. It has no US, FR, AC,
TC, option or constraint rows, so the user-story, functional-requirement and six
test-coverage rules have nothing to check. The gates that apply are ID format and
uniqueness, cross-references, link validity and terminology. The authoring agent
ran this base pass. Independent reviewers ran the seven analyses (SR-483 to
SR-489).

Verdict: ACCEPT WITH FINDINGS (no blocking findings).

## Method

- ID format: `ADR-013` matches `^[A-Z]{2,4}-[0-9]+$`. The coordinator reserved
  it for #211.
- Local item ids are each defined once, in sequence, with no gaps:
  - `R-01` to `R-10`, in §1;
  - `O-01` to `O-27`, as §3 headings;
  - `C-01` to `C-16`, in the §4 table;
  - `Q209-1` to `Q209-7`, `Q210-1` to `Q210-4`, and `OQ-1` to `OQ-3`, in §8.
- Every other occurrence of these ids is a cross-reference. Every cross-reference
  resolves to a defined id.
- The relationship targets resolve:
  - `ADR-010` resolves to `spec/decisions/ADR-010-observed-architecture-baseline.md`.
  - `ADR-009` resolves to `spec/decisions/ADR-009-graph-path-witness-content.md`.
  - `ix://agent-ix/quire-specification/AD-016` resolves to the accepted QSpec
    assurance record on origin/main.
- `spec/spec.md` gains one `contains` relationship and one index row. Both
  resolve to the new file.
- The one Mermaid block, the §4 flowchart, contains no `;`.
- Terminology: `CanonicalDigest`, `DeclarationKey` and `CheckedPackage` keep
  their existing names. No rename is proposed.
- Wording is current-state. No compatibility, migration or fallback design
  appears. Old-version support appears only as refusal (§5).
- `quire validate --scope <repo> <ADR-013> spec/spec.md --strict --summary`
  reports 2/2 grammar-clean.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The checklist rules for US, FR and TC quality and the six test-coverage rules do not apply, because the ADR has no acceptance criteria. They are recorded as not applicable, not as passed. Normative requirement text belongs to #213 and #231. | ADR-013 |
| FND-002 | low | The item-id schemes `R-`, `O-`, `C-`, `Q209-`, `Q210-` and `OQ-` are local to ADR-013 and are not catalog id kinds. Layer 2 tickets must cite them in the form `ADR-013 O-nn` to stay unambiguous next to ADR-010 `OBS-` and `DA-` ids. Fixed in round 1: the Decision section states this citation form. | ADR-013 §3, §4, §8 |

## Round 2 (commit 0042691)

Re-checked after the round-1 revision. The id kinds are now `R-01` to `R-10`,
`O-01` to `O-27`, `C-01` to `C-25`, `S-1` to `S-6`, `QC-1` to `QC-7`, `Q209-1`
to `Q209-7`, `Q210-1` to `Q210-4`, `Q222-1` and `Q222-2`, `Q229-1`, and `OQ-1`
to `OQ-4`. Each runs in sequence with no gaps and is defined once. Every
cross-reference resolves. The Mermaid block still contains no `;`. A scan for
compatibility, fallback, migration and transitional wording finds none in the
decision text. `quire validate --strict --summary` on ADR-013, spec.md and the
eight review files reports 10/10 grammar-clean.

- FND-001: unchanged, not applicable.
- FND-002: resolved. The Decision section states the citation form and now
  lists every local id kind.

Round-2 verdict: ACCEPT WITH FINDINGS (FND-001 only).

## After round 2

The round-2 medium and low findings in all eight reviews are resolved in the
ADR. The same commit adds coordinator-requested answers: ADR-012 §13.2
questions (OQ-5, O-10, O-14, O-17, O-19; O-20 decided in #222) and the nine
ADR-011 questions to #211 (§3.1, T-1 to T-9, with QC-10 to QC-12 and Q209-8).
§3.1 was added after round 2 and has had no separate review round; the two-round
limit applies.

## Round 3 (commit 4152eb8)

PR #236 re-review of the delta 5609e3a..4152eb8 only, against ADR-011 at
22fa948 (PR #235) and ADR-012 at 10664aa (PR #234). The first PR review
(head 5609e3a) returned CHANGES with findings H1 to H3, M1 to M9, lows and
nits. Line numbers below are ADR-013 lines at 4152eb8.

Round-1 PR findings, re-checked:

- H1 resolved. O-25 carries `ReplaySource` (`Witness` or `Input`, never
  both) and an `Input` replay settles `reproduced-without-witness`. ADR-011
  E9 at 22fa948 states the same rule.
- H2 resolved. `CatalogCode` and the category type are in F `diagnostic`;
  member, variant and unit identities are opaque kernel digests (QC-15).
- H3 partly resolved. `from_bytes` is removed, `from_hex` is no longer public,
  both identities are minted from a digest, and T-3 holds a `WireNodeId`.
  The enforcement claim and the RT and CG path remain open (PR2-H1).
- M1 to M9 resolved: T-8 matches O-26; the capability vocabulary is QSpec
  #134's; the O-26 request adds the #231 envelope members; QC-18 gives node
  ids package scope; O-09 and QC-14 record the ADR-011 E7 note; simulation
  converges into S6a; Q209-5 quotes FB-05; O-14 and QC-19 match ADR-012
  (in-place node-kind revision); T-4 uses `Locus`.
- Lows and nits resolved, except that Q209-2 still cites ADR-011 §1 (PR2-L3).

Coordinator cross-ADR checks: `VerifiedPackage` in layer-3 `library` agrees
with ADR-011 I2, §4 and §6.1. The layer-6 `replay` facade as the TK-01 entry
agrees with ADR-011 §6.1. The clause kind in the layer-3 `check` core agrees
with ADR-011 §6.1 and ADR-012 S5. `Relation` → `Refused(FamilyNotNativelyEvaluable)`
agrees with ADR-012 §2, §8 and §13.5 (but see PR2-M3). X-1 = #213 S-1 gated by
TK-10 and QC-15 agrees with ADR-011 X-1 and T-6. `102c8bb` is an ancestor of
22fa948.

New findings:

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| PR2-H1 | high | The `NodeKey` minting rule is still unenforceable across crates, and RT and CG have no legal source of `NodeKey`s. O-04 says "the ADR-011 T-12 API-surface check fails any other caller", but ADR-011 T-12 checks only that CG uses the `replay` facade and the backend `cargo tree` direction. No check covers RT or CG calling the kernel constructor, or QSL modules other than `check` calling it. O-04 also says RT and CG "receive `NodeKey`s only in the packets and requests they are handed", while every node id read from a wire is a `WireNodeId`. The kernel enum and sum shape (O-14, T-6) and the C-11 reconstruction to kernel `Value`s need a `NodeKey`, so CG and RT must call the constructor. Fix: either give the kernel enum and sum payload an opaque digest newtype, like `VariantId`, so RT and CG never need a `NodeKey`, or route every wire → `NodeKey` step through the `replay` lookup. Then name a check that exists, or ask ADR-011 to extend T-12 to kernel constructor callers. | ADR-013 166, 178, 314, 654, 676 · ADR-011@22fa948 383, 865 |
| PR2-M1 | medium | `PackageNodeKey` has two definitions. ADR-013 T-3 has `PackageNodeKey{package: package_id, node: WireNodeId}`. ADR-011 at 22fa948 E3 has `PackageNodeKey{package: package_id, node: NodeKey}`, and ADR-011 I2 says a `WireNodeId` becomes part of a `PackageNodeKey` only after the E4 lookup. Fix: one ADR changes to match the other. | ADR-013 168, 651 · ADR-011@22fa948 185, 297 |
| PR2-M2 | medium | `CounterexamplePacket.source: ReplaySource` replaces the accepted AD-016 arrow 6 output and Replay-ownership Packet row (`witness: Option<Witness>`), and no QC amends AD-016. QC-6 and QC-8 amend FR-331 and FR-323 only, QC-7 covers only the WP9 parity carrier, and OQ-3 accepts QC-13 to QC-17. ADR-011 E8 at 22fa948 still outputs `CounterexamplePacket{witness: Option<Witness>}`. Fix: add the Packet row change to QC-7, or add a QC, and have ADR-011 E8 cite it. Or keep `witness: Option<Witness>` plus a stored input and state the one-of rule over those fields. | ADR-013 568, 577, 806, 807, 863 · AD-016 Arrow 6, Replay ownership (Packet) · ADR-011@22fa948 236 |
| PR2-M3 | medium | `FamilyNotNativelyEvaluable` is placed in the kernel `Refusal`, but the kernel `Refusal` "carries only the kernel's own typed cause", and T-6 moves the family cause `WrongSnapshotCause` out of it. A family-dispatch cause makes the leaf kernel name a QSL family concept. The same question applies to family causes in S6a outcomes: `Outcome::Refused(Refusal)` cannot hold a `value::expression` cause once `WrongSnapshotCause` leaves. Fix: state how an S6a result carries a non-kernel refusal cause (for example an S6a result type that wraps the kernel `Outcome`), and put `FamilyNotNativelyEvaluable` there. ADR-012 §2, §8 and §13.5 must follow. | ADR-013 366-368, 401-402, 654, 839 · ADR-012@10664aa 239-240, 596, 852 |
| PR2-L1 | low | The sibling-head pin is stale. Context pins ADR-011 at f781e32 and ADR-012 at 43677c9, but the delta cites text that exists only later (the `replay` facade, the ADR-011 E9 `ReplaySource` rule and the T-12 API-surface check). Fix: pin 22fa948 and 10664aa, or the merged heads. | ADR-013 60-61 |
| PR2-L2 | low | O-04 says "the dependency compiled from source under the ADR-011 M-3 ruling". ADR-011 M-3 is "Add S2 `forms`". The rule is ADR-011 E4 and I2. | ADR-013 168 · ADR-011@22fa948 185, 232 |
| PR2-L3 | low | Q209-2 still says ADR-011 §1 answers which stage owns `ResolvedSourcePackage`. ADR-011 §1 does not name it. ADR-011 §8 maps C2 to I2 and `library`, as O-15 now says. | ADR-013 825, 325 |
| PR2-L4 | low | QC-15 now lists six named types (`EffectiveId`, `UniverseId`, `ObjectId`, `UnitId`, `VariantId`, `MemberId`) "and no other". OQ-3 records the owner's acceptance of QC-13 to QC-17 before this list existed. Fix: record the owner's confirmation of the widened QC-15 in OQ-3, or in TK-10. | ADR-013 814, 863, 885 |

Mermaid still has no `;`. SR-482 to SR-489 are unchanged and collision-free.
`quire validate --strict --summary` on ADR-013, spec.md and the eight review
files reports 10/10 grammar-clean.

Round-3 verdict: CHANGES. PR2-H1 blocks. PR2-M1 to PR2-M3 are cross-ADR
contradictions that must be resolved in one of the three ADRs before the #212
gate. PR2-L1 to PR2-L4 are low.

## Author fixes (round 3)

Rulings by Agent A under the owner's delegation (2026-09-19). Sibling pins:
ADR-011 at `1666d02`, ADR-012 at `eecf825`.

| ID | Disposition | Where |
| --- | --- | --- |
| PR2-H1 | Fixed. RT and CG never mint a `NodeKey` or read one from a wire; every wire id is a `WireNodeId`, converted only by QSL lookup (E4 for imports, the layer-6 `replay` facade at E9 for packets and requests). RT holds `NodeKey`s only as in-process values from S6a. The kernel enum and sum payload carry `VariantId` only. C-11 reconstructs arguments keyed by `WireNodeId`. Enforcement cites the widened ADR-011 T-12 API-surface check (a) CG → `replay` only, (b) `check` → `NodeKey`, (c) `model` → `EffectiveId`. | O-04, O-05, O-14, O-25, T-6, C-11 |
| PR2-M1 | No change. ADR-013 T-3 (`node: WireNodeId`) stands; ADR-011 E3 follows it. | T-3 |
| PR2-M2 | Fixed. QC-20 amends AD-016 arrow 6 and the Packet row to `source: ReplaySource`; it joins the TK-10 amendment PR. Counts now twenty QSpec changes, six AD-016 amendments. ADR-011 E8 follows. | O-25, QC-20, OQ-3, TK-10, Context, Consequences |
| PR2-M3 | Fixed. The kernel `Refusal` holds kernel causes only. S6a returns the layer-3 `check`-core `FamilyOutcome { Evaluated(kernel::Outcome), Refused(FamilyRefusal) }`; `FamilyRefusal` carries `FamilyNotNativelyEvaluable` and maps to category `refusal` in F `diagnostic`. ADR-012 follows at `eecf825`. | O-16, O-17, T-6, Q210-3 |
| PR2-L1 | Fixed. Pins are ADR-011 `1666d02` and ADR-012 `eecf825`. | Context |
| PR2-L2 | Fixed. The dependency compile cite is ADR-011 §1 S4 and E4/I2. | O-04 |
| PR2-L3 | Fixed. Q209-2 cites ADR-011 §8 for `ResolvedSourcePackage`. | Q209-2 |
| PR2-L4 | Fixed. OQ-3 records the confirmation of QC-15's six types and no others. | OQ-3 |
| ADR-011 R3 low (FB-05 quote) | Fixed. Q209-5 paraphrases FB-05 with a §3 cite. | Q209-5 |
| ADR-011 R3 low (IR #139) | Fixed. IR PR #139 is merged at `954c2f2`. | Context, TK-10 |
| ADR-011 R3 low (TK-01) | Fixed. The skeleton spine (ADR-011 T-2) lands the `replay` facade first; #214 widens it per family. | TK-01 |
| ADR-011 §4 dependency binding | No change. ADR-013 lists no E4 or E9 refusal codes, so `DependencyIdentityMismatch` is not added. | none |

## Round 4 (commit 02a504f)

Final PR #236 check of the delta c7d8df1..02a504f, against ADR-011 at 1666d02
and ADR-012 at eecf825.

- PR2-M1 to PR2-M3 and PR2-L1 to PR2-L4 are fixed. `PackageNodeKey{package,
  node: WireNodeId}` is the same in all three ADRs (ADR-011 I2 and E3). QC-20
  amends the AD-016 Packet row, and ADR-011 E8 and E9 cite it. `FamilyOutcome
  { Evaluated(kernel::Outcome), Refused(FamilyRefusal) }` is in the layer-3
  `check` core in all three ADRs (ADR-011 §6.1 layer 3). ADR-011 T-12 has the
  three-part API-surface check. ADR-011 §4 `DependencyIdentityMismatch` agrees
  with O-04, O-26 and T-2.
- PR2-H1 is fixed for CG, for the kernel payloads (`VariantId` only) and for
  enforcement (T-12 (b) and (c)). The RT half is not fixed:

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| PR3-M1 | medium | O-04 says RT holds `NodeKey`s "only as in-process values that QSL passes to it through S6a", and ADR-012 keys RT's function lookup by `NodeKey`. The ADR-011 §7.1 crate graph has no QSL → RT edge, and S6a never calls RT. RT runs inside CG-generated harnesses built from IR wire data, so its only node ids are wire ids, and it would have to mint `NodeKey`s, which T-12 (b) forbids. Fix: key RT's function lookup by `WireNodeId` (or a CG-assigned index), and say RT holds no `NodeKey` (ADR-013 O-04, ADR-012 §9). | ADR-013 166 · ADR-012@eecf825 666, 859 · ADR-011@1666d02 §7.1 |
| PR3-N1 | nit | "F `diagnostic` maps it (`FamilyRefusal`) to category `refusal`": F sits below layer 3 and cannot name `FamilyRefusal`. O-17 has the layering right: `FamilyRefusal::catalog_code()` yields the code, and F maps the code to its category. Same wording in ADR-012 §2 and §13.5. | ADR-013 368 |

`quire validate --strict --summary` reports 10/10 grammar-clean.

Round-4 verdict: CHANGES (PR3-M1 only: two sentences in ADR-013 O-04 and ADR-012 §9 and §13.5).

## Author fixes (round 4)

Ruling by Agent A (2026-09-19): RT holds no `NodeKey`. ADR-012 pin stays at
`eecf825` until its next head is given.

| ID | Disposition | Where |
| --- | --- | --- |
| PR3-M1 | Fixed. RT and CG hold only `WireNodeId`s; only QSL (E4 and the layer-6 `replay` facade) converts one to a `NodeKey`. RT runs inside CG-generated harnesses built from IR wire data, and ADR-012 keys RT's function lookup by `WireNodeId`, an id lookup that keeps R-06. The S6a in-process sentence is removed. | O-04 |
| PR3-N1 | Fixed. `FamilyRefusal::catalog_code()` yields the code and F `diagnostic` maps the code to category `refusal`; F never names `FamilyRefusal`. | O-16 |
