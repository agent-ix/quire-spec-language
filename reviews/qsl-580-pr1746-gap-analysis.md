---
id: SR-1746002
title: "gap analysis of PR #1746 (QSL-580s): STD-526 template clock_place nodes"
type: SpecReview
analysis: gap-analysis
date: 2026-10-28
scope: "agent-ix/quire-spec-language; PR #1746 at 385603d47 against STD-526 (QSpec 7d8d2090): FR-322-AC-148..150, FR-415, FR-230-AC-49..51, timed-slot-selector-vectors.json"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-136
    type: reviews
---
# SR-1746002: gap analysis of PR #1746 (QSL-580s)

## Summary

Ticket: QSL-580. Head: 385603d47.

Covered:
- FR-230-AC-49 / FR-322-AC-148/149: `a_clock_collection_emits_one_template_place` checks one place, the symbolic Integer selector, the rows and the empty `guards`.
- FR-230-AC-50: `a_clock_model_with_a_guard_is_not_emitted`.
- FR-322-AC-150 intake: the intake proof refuses an index literal.
- FR-230-AC-51 (presence through the place identity) is not in this slice. The STD-526 presence forms stay refused, as recorded in SR-1743002 FND-003.

Gaps:
- **FND-001.** No test compares the emitted place against the published vectors. My probe found the body canonical-equal to SL-08 after substituting the model digest, the identities and the bound. A test reading `timed-slot-selector-vectors.json` from QSPEC_DIR, as the C14/I2 tests read the fixtures, would pin SL-01, SL-05 and SL-08 instead of the hand-written path and row assertions.
- **FND-002 (latent).** `element_type` does not strip `Option<..>`. For an `o: Option<Sequence<Clock>>` or `Option<Timer>` place, the slot row's `declaring_type` and `member_type` would be the whole `Option<..>` string. S3 refuses those places today (`TimedNotChecked`, STD-493 item 17, `a_record_clock_under_an_optional_or_collection_field`), so nothing is emitted wrong now. FR-230-AC-19 lists `o: Option<Sequence<Clock>[0, 2]>` as a place, so the row rule needs the Option case when those places are admitted.
- **FND-003.** The union-case row identity `<declaring type>/union/<case>` is pinned by neither a vector nor a test assertion against QSpec. FR-322 says only "resolved member identity". IR-1083 will decide the reader join, so a mismatch would surface only at read-back.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No conformance test against timed-slot-selector-vectors SL-01/05/08; probe shows SL-08 canonical-equal | FR-322-AC-148 |
| FND-002 | low | Latent: `element_type` ignores `Option<..>`, so rows would be wrong once Option-wrapped clock collections are admitted | FR-230-AC-19, FR-322 |
| FND-003 | low | The union-case row identity `<type>/union/<case>` is unpinned by spec, vector or read-back | FR-322 |

## Verdict

Mergeable. All three findings are low.

## Dispositions

Round 1, head addc801df.

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | `conformance_the_template_places_match_the_slot_selector_vectors` compares the emitted `ts`, `g` and `s` places with SL-01, SL-05 and SL-08 from QSPEC_DIR (body and dependency digests); passes against QSpec 161d231d. The v and q presence places are added in #1763 |
| FND-002 | fixed | `element_type` reads through `Option<..>`; unit test `the_element_type_reads_through_an_option` passes |
| FND-003 | fixed | The union-case row identity is filed on STD-493 (comment a7060596) |
