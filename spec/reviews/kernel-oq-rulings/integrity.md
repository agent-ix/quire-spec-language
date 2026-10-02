---
id: SR-510
title: "Kernel-convergence rulings OQ-A to OQ-F integrity review (PR #351)"
type: SpecReview
analysis: integrity
scope: "PR #351 diff: ADR-011 §6.1/§7.1, ADR-013 O-05, O-13, O-14, T-6, C-26, QC-15, QC-21 to QC-23, §8 OQ-A to OQ-F; FR-067, FR-084, FR-088, FR-091; TC-252, TC-398, TC-409, TC-410; spec/spec.md; spec/tests.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: reviews
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-088
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-091
    type: reviews
---
# SR-510: Kernel-convergence rulings OQ-A to OQ-F integrity review

## Summary

Checks consistency within and across ADR-011, ADR-013, FR-067, FR-084,
FR-088, FR-091, the TCs and the indexes after PR #351. It also sweeps the rest
of `spec/` for statements the rulings contradict, by grepping `VariantId`,
`UnitId`, `UniverseId`, `ObjectId`, "variant index", "1, F", "FR-091-OQ-8",
"universe" and "as amended by QC-15".

The sweep finds OQ-A applied everywhere: ADR-011 §6.1 and §7.1, FR-067,
FR-091, TC-398 and `tests.md` no longer say "1, F", and nothing outside the
PR still states the old layer-2 allow-list. O-14's "never a variant index" is
amended consistently in ADR-013, FR-088 and TC-252. Four areas are
inconsistent: the AD-016 amendment bookkeeping (QC-15 against QC-22, and the
places that count or list amendments), TC-252's recorded pass status, the
OQ-A rationale's reading of FB-01, and three stale pointers.

## Verdict

**REVISE.** FND-001 to FND-005 (medium) leave ADR-013's own amendment record
and the Test Matrix saying things that are no longer true. Each fix is local
text.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | QC-15 (`ADR-013:1026`) is rewritten in place, and QC-22 (`:1033`) then records the same change as a new amendment. QC-15 is an accepted AD-016 amendment: the OQ-3 row (`:1079`) says it was filed in TK-10, and AD-016's kernel row at `2449ceb` (`spec/assurance/AD-016-semantic-family-extension-path.md:356`) carries QC-15's original text, "each is an opaque 32-byte digest newtype". QC-15 now describes shapes AD-016 never accepted, and adds "QC-22 amends it". Two rows therefore state one change, and QC-15 no longer describes what QSpec holds. **Fix:** restore QC-15 to the text AD-016 accepted. Put the new shapes (`UniverseId` domain, `ObjectId` authored UTF-8 with its string constructor, two-domain `UnitId`, `VariantId` as the FR-141 member key, enum rank) in QC-22 only. | ADR-013:1026, :1033, :1079; QSpec AD-016:356 @2449ceb |
| FND-002 | medium | QC-22 says it "closes [AD-016's] `OPEN — decided in QSL #213` cell with the OQ-B, OQ-C and OQ-F rulings". That AD-016 cell (`AD-016:356`) decides two things: the FR-201 domain of `UniverseId`, `ObjectId`, `UnitId`, `VariantId` **and `MemberId`**, and "whether ADR-011 T-12 limits their minting". The rulings settle neither `MemberId`'s domain nor any T-12 minting rule. ADR-011 `:747-750` ("Kernel identity constructors have one caller each") still lists only `NodeKey`, `EffectiveId` and `PopulationId`. QC-22 overclaims. **Fix:** make QC-22 narrow the OPEN cell to what remains open (`MemberId`'s domain; the T-12 minting callers for `UniverseId`, `UnitId`, `VariantId`, `ObjectId`), or decide them in O-14/T-6 and extend ADR-011 `:747-750`. See SR-511 FND-006 for why the minting limit matters. | ADR-013:1033; ADR-011:747-750; QSpec AD-016:356 @2449ceb |
| FND-003 | medium | Stale amendment counts and lists. ADR-013 `:47-50` still says "Seven cells of this record differ from accepted AD-016 text" and names QC-13 to QC-17, QC-20 and QC-21. QC-22 now amends the kernel row too. ADR-011 `:603` (K row), `:660`, `:790` and `:964` still say the kernel is "the AD-016 Shared-type row as amended by QC-15 and QC-21 (TK-10)", which leaves out QC-22. None of the QC-21 to QC-23 rows names a filing ticket, so "(TK-10)" is also wrong for QC-21 and QC-22. **Fix:** in ADR-013 `:47-50`, update the count and list to include QC-22. In the four ADR-011 places, write "as amended by QC-15, QC-21 and QC-22". Drop "(TK-10)" or name the ticket that files QC-21 and QC-22. | ADR-013:47-50; ADR-011:603, :660, :790, :964 |
| FND-004 | medium | TC-252's recorded pass status is stale. The PR changes TC-252 step 4 to require that each `VariantId` is "the FR-141 enum member node key". The test that backs it (`src/check/identity.rs:877-908`, `c26_sum_preserves_node_id_and_mints_position_independent_variant_ids`) checks `mint_variant_id`, which uses QSL's private `"sum-variant-member"` preimage (`src/check/identity.rs:570-572`), and ADR-013 OQ-F (`:1097`) says that preimage is replaced. `spec/tests.md:138` still marks TC-252 `✅ Passed locally; #300`. FR-088's Status (`:231-240`) still says AC-10 is backed by TC-252. FR-088's Status also says "S-3b implemented by #300", while `spec/spec.md:468` says FR-088 is "not yet implemented". **Fix:** set TC-252 to `🚧 Planned; QSL-131 (step 4 FR-141 member key; step 6)` or split steps 4 and 6 into TC-409. Update FR-088 Status to list AC-10 (the FR-141 part) and AC-11 as remaining under QSL-131, and make `spec.md:468` agree with it. | spec/tests.md:138; FR-088:231-240; spec/spec.md:468; TC-252 steps 4, 6; src/check/identity.rs:570, :877-908 |
| FND-005 | medium | The OQ-A rationale misreads FB-01. ADR-011 FB-01 (`:489`) forbids "any stage after S1" from reading "source text, CST, token text or display strings to recover semantics". S2 (`forms`) comes after S1, and it is the stage that turns literal token text into a `quire_exact::Integer`. ADR-013 OQ-A (`:1092`), ADR-011 `:693-694` and FR-091 `:242-247` say the edge exists so that "E3 ... never re-reads token text (FB-01)". Read literally, FB-01 forbids the reading OQ-A puts in S2. ADR-011's E2 row (`:355`) also carries "Declared bounds and extents ... as syntax", which E3 must then interpret. FR-091-OQ-9 (`:437`) names that tension, but ADR-011 and ADR-013 do not. **Fix:** state in the OQ-A row and at ADR-011 `:693` that S2, the CST consumer, is the stage that interprets literal tokens, and that FB-01 binds S3 onward (or amend FB-01's row to "after S2"). Cross-reference FR-091-OQ-9 from OQ-A as the open case for bounds. | ADR-011:355, :489, :690-698; ADR-013:1092; FR-091:242-247, :437 |
| FND-006 | low | Dangling ID. The ADR-013 OQ-A row (`:1092`) says "FR-091-OQ-8 asked the same question", and this PR deletes FR-091-OQ-8 from FR-091. No other file defines it. **Fix:** drop the clause, or write "FR-091's layer-2 question" without the deleted ID. | ADR-013:1092; FR-091:431-444 |
| FND-007 | low | The code-state notes are uneven. O-05's note (`:226-230`) records that the kernel `UniverseId`/`ObjectId` are digests in `quire.universe/v1`/`quire.object/v1`, and O-14 records `key.rs:103`. Two other deviations go unrecorded: the kernel `UnitId` is one digest in `quire.unit/v1` (`quire-exact/src/identity.rs:121-127`), against OQ-B's two domains, and the kernel `VariantId` is in `quire.enum-variant/v1` (`quire-exact/src/identity.rs:129-134`), against OQ-F's `quire.checked-semantic-node/v1`. Both identity doc comments also still cite T-6 for shapes T-6 no longer states. **Fix:** add the two sites to the T-6 or O-14 code-state sentence, with "Remaining work: QSL-131". | ADR-013:226-230, :360, :848; quire-exact/src/identity.rs:121-134 |
| FND-008 | low | FR-088 `:64` cites "ADR-013 §8 QC-15 (`:848`; ...)". Line 848 is now T-6, and QC-15 is at `:1026`. The pointer was already wrong before the PR (QC-15 was at `:989`), and the PR's insertions moved it further. **Fix:** cite "ADR-013 QC-15" by ID with no line number. | FR-088:64; ADR-013:848, :1026 |
