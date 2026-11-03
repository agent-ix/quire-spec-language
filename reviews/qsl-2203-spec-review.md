---
id: SR-2203001
title: "spec-review of QSL source_integrity spec (quire-spec-language PR 2203)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language; PR 2203 head a06302780; FR-528, FR-071, FR-098, FR-350, FR-518, TC-1323..TC-1325"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-528
    type: references
---

# Spec review of QSL source_integrity spec (PR 2203)

## Summary

Tickets: QSL-1559, QSL-1560, QSL-1562. Integrity, EARS, failure-domain, object and criterion-strength lenses applied inline. `quire validate` exits 0 on all 9 touched files; FR-528 and TC-1323..1325 are unused on origin/main and no open QSL PR (2201, 2202) touches them. `quire matrix`: new ACs untagged, expected for a spec-only PR. Verified against QSpec main: FR-322-AC-319..341, FR-323-AC-29..35 (RP-R1..R7) and FR-336-AC-9/10 say what the PR cites; the FR-071 two-phase order (syntax of every claim, then per entry bytes then digest, missing_import wins when bytes are absent, first failing entry decides) matches FR-323-AC-34/35; AC digests recomputed (`a\r\nb\n` 953bba9a...f39d, empty e3b0c442...b855). FR-001 is verbatim (BOM, NUL, invalid UTF-8 refused), so for an admitted Source the claim is the raw bytes digest. Revision-label removal narrows nothing ticketed: FR-001 already drops SourceIdentity to authority and identity.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | FR-528 puts the SHA-256 in qsl-foundation, but NFR-015-AC-2 requires `cargo tree -p qsl-foundation --no-default-features` to list exactly quire-exact, quire-semantic-value, serde, thiserror and the ix-trace-rs proc macro, and NFR- | FR-528-write-a-source-integrity-claim-on-every-source-reference-of-an-emitted-checked-package.md | missing-requirement |
| FND-002 | medium | The claim is the digest of the bytes compiled only if the attached digest came from the Source that was compiled. SourceDigest::of is public and the builder checks only pair membership and conflicts (AC-4, AC-5), so a caller can a | FR-528-write-a-source-integrity-claim-on-every-source-reference-of-an-emitted-checked-package.md | missing-requirement |
| FND-003 | medium | For a source admitted verbatim the claim is exact (FR-001 refuses BOM, NUL and invalid UTF-8 and keeps the text, so text bytes equal the supplied bytes). FR-528 is silent on an extracted native body (FR-030/FR-031): the reference  | FR-528-write-a-source-integrity-claim-on-every-source-reference-of-an-emitted-checked-package.md | missing-requirement |
| FND-004 | medium | AC-9 and AC-10 are not gated, so every existing emit caller (CLI, FR-099 library import, FR-518 handoff writer, tests) that does not attach digests starts refusing, before the wire can carry the claim (IR-1273 gating). Signatures  | FR-528-write-a-source-integrity-claim-on-every-source-reference-of-an-emitted-checked-package.md | missing-requirement |
| FND-005 | low | Dropping the revision labels from the wire narrows no ticketed behavior: FR-001 already says SourceIdentity drops to authority and identity (AC-6..11) and FR-071 already said two labels; FR-098 now agrees. The code (SourceIdentity | FR-071-implement-typed-replay-request.md | wrong-requirement |
| FND-006 | low | FR-350 Settlement defines tool-failure as holding a panic message, fault or I/O error; a cause `corpus-decode` and a pointer are a new field no other text defines. Extend the Settlement text. | FR-350-run-the-conformance-corpus-against-qsl.md | missing-requirement |
| FND-007 | low | AC-5 and AC-8 bundle two claims each. FR-350-AC-15 cannot fail by itself: it says integrity failures are settled by the typed refusal and never by the adapter, and the corpus holds none. | FR-528-write-a-source-integrity-claim-on-every-source-reference-of-an-emitted-checked-package.md | wrong-requirement |

## Verdict

Changes requested, not mergeable: FND-001 (high) must be amended in this PR. FND-002 to FND-004 are medium and need the author or a planner ruling (FND-004: gate the emit refusal with the lock bump or list the callers). Peter ruled source_integrity in scope; nothing here defers it.
