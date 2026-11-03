---
id: SR-2211001
title: "spec-review of source_integrity removal (quire-spec-language PR #2211)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language; PR #2211; 12 files: FR-528 (retired, AC-1..32), FR-071, FR-098, FR-350, FR-518, NFR-015, NFR-016, TC-1110, TC-1313, TC-1323..1325"
review_set: subset
---

# Spec review of source_integrity removal (quire-spec-language PR #2211)

## Summary

Tickets: QSL-1562, QSL-1559, QSL-1560. Checks: grep for source_integrity, source-integrity, SourceDigest, sha2 and FR-528 in spec/ leaves only the retired FR-528 stub, TC-1323/1324 stubs, the spec.md index row, retirement notes and unrelated SHA-256 uses (FR-188, ADR-024); NFR-015 row, AC-2 and AC-1 are consistent without sha2 and the hashbrown and serde_json correction is right (Cargo.lock: qsl-foundation depends on hashbrown, ix-trace-rs, quire-exact, quire-semantic-value, serde, serde_json, thiserror); NFR-016-AC-9, NFR-015-AC-11, FR-098-AC-30/31, FR-071-AC-18..32, FR-350-AC-15, FR-518-AC-9/10 retired with ids kept; FR-071 is back to two labels, no digest, checks no source byte, and FR-071-AC-5 is the unconditional completeness rule again; FR-518 counts 30 = 20 reader + 10 selection recount correctly; the two-label fixes, ADR amendment notes and FR-001 section are untouched; `quire validate` passes on all touched files; no open QSL PR touches these files (2204 is FR-501/FR-525; 2210 is a code PR).

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | low | The spec now says the corpus is 30 cases (20 reader, 10 selection; I recounted rows 1-13, 14b, 15b, 16-27, 29-31) with no case 28m and no emitted claim, but the code still writes source_integrity and 31 cases (src/checked_handoff. | spec/functional/FR-518-write-the-adverse-mutation-corpus-from-checked-packages-and-name-the-destination-of-every-legacy-case.md | missing-requirement |
| FND-002 | medium | Carried from the merged #2203 adapter text: FR-350-AC-12 and TC-1325 row 2 map `sources` of the package and of each dependency, but the published `$defs.ReplayRequest` has 13 members and no `package` (QSpec FR-336 lists one in its | spec/functional/FR-350-run-the-conformance-corpus-against-qsl.md | wrong-requirement |
| FND-003 | low | TC-1313's procedure rows jump from 8 to 11 (rows 9 and 10 deleted, matching retired FR-518-AC-9/10); keep them as retired rows or note the gap so the row numbers keep their AC mapping. | spec/test-cases/TC-1313-write-the-adverse-mutation-corpus-from-checked-packages-and-name-the-destination-of-every-legacy-case.md | wrong-requirement |

## Verdict

Mergeable. The retirement is complete and consistent; FND-001 and FND-002 are owner calls, not defects of the revert.
