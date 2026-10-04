---
id: SR-1293
title: "Gap analysis of quire-spec-language A3c: recomputed identities refuse content-mismatch"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@b3a2a6e3261429efffc7573d5c94ecfdb2146b09; PR #629, git diff origin/main...HEAD: acceptance criteria FR-056-AC-2, FR-056-AC-12, FR-071-AC-6 (and FR-071's carrying-both-digests clause), FR-087-AC-3, FR-087-AC-11, FR-106-AC-3, FR-106-AC-7, FR-116-AC-4, FR-122-AC-5, FR-261-AC-2, ADR-015 D-4 rule 7, against the tests that trace them"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-071
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-261
    type: reviews
---
# Gap analysis of quire-spec-language A3c

## Summary

Ticket: QSL-395 (A3c, MS-03). PR #629 at b3a2a6e32.

Acceptance criteria checked against their tests:

| AC | Test | Backed |
| --- | --- | --- |
| FR-056-AC-2 (parsed document, stale digest) | `a_parsed_document_under_another_digest_refuses_content_mismatch` (intake.rs) | Yes. Exact cause with `selected` and `recomputed` and the exact detail string. |
| FR-056-AC-2 (lone surrogate) / FR-056-AC-12 | intake.rs:6035, :6619 | Yes. Still `ByteDigestMismatch` with the raw digest as `actual`. |
| FR-106-AC-3 | `a_parsed_document_under_another_digest_refuses_content_mismatch` (observation.rs); `tc465_row4_...` (state_clauses.rs) | Yes. Record fields `selected` and `recomputed` asserted with exact values. |
| FR-106-AC-7 (row 29) | `tc465_row29_a_digest_mismatch_beats_an_unknown_member` | Yes, cause only (the row asks for the cause only). |
| FR-071-AC-6 | `tc_186_byte_provision_is_digest_only_complete_and_bounded` | Variant only: `ContentMismatch { .. }` and raw `ByteDigestMismatch(_)`. See FND-001. |
| FR-116-AC-4 / FR-122-AC-5 | `edited_invocation_bytes_refuse_content_mismatch` (both files) | Yes for the variant, code and `stale_dependency/content-mismatch` prefix. |
| FR-087-AC-3 / AC-11 | `a_resolved_library_lock_is_a_pinned_request`; checked_v2 TC-253 test | Yes. `LibraryCause::ContentMismatch`; `PinMismatch` carries both. |
| ADR-015 rule 7 | execute/tests.rs:1074-1130 | Yes. Destructures `identity`, `requested`, `recompiled` and asserts them. |
| FR-261-AC-2 | none | No. See FND-002. |

Tests run: see the code review (SR-1292) Verdict.

## Verdict

Two medium coverage gaps. Neither is a wrong behaviour today; both leave a
clause the PR changed with no oracle that could fail.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-071 now says a `sha256-jcs` byte-provision entry refuses `content-mismatch` "carrying both digests", but every oracle on `ReplayRequestRefusal::ContentMismatch` matches `{ .. }`. Swapping `selected` and `recomputed`, or reporting the declared digest twice, passes every test. Assert both values in the TC-186 test (request.rs:963-967) with the document's real recomputed `sha256-jcs` digest. | qsl-replay/src/request.rs:963-967; qsl-replay/src/spine/clause/tests/frame_replay.rs:878-883; qsl-replay/src/spine/clause/tests/state_clause_replay.rs:851-856; spec/functional/FR-071-implement-typed-replay-request.md:109 |
| FND-002 | medium | FR-261-AC-2 (TC-733 step 2) has no test. No test carries `#[trace("TC-733", "FR-261-AC-2")]` and none admits a 100,000-deep snapshot on a 512 KiB stack. The PR changes this AC's expected cause to `content-mismatch` and the PR body lists it among the changed sites, but nothing checks it. Write the test TC-733 step 2 describes, asserting `content-mismatch` with both digests under the other digest. | spec/functional/FR-261-read-other-untrusted-json-at-any-depth.md:63; spec/test-cases/TC-733-library-and-observation-reads-judge-deep-documents-on-content.md:36-43 |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-003 | medium | The fix round gives FR-071-AC-5 (and TC-186 step 5) an exact refusal: `missing_import`/`missing-selection`, naming the requested digest record and where the package reference names it. `IncompleteByteProvision` now has `requested` and `named_by` and maps to `Code::MissingImport`, but every oracle on it still matches `IncompleteByteProvision { .. }`, and none checks `code()` or the rendered cause. Reverting the code to `MissingDeclaration`, or dropping or swapping `named_by`, passes every test. Assert the code, the `missing_import/missing-selection` prefix and both fields in the TC-186 test. | qsl-replay/src/request.rs:886, :1019; qsl-replay/src/execute/tests.rs:486-491; spec/functional/FR-071-implement-typed-replay-request.md:136 |

## Dispositions

Round 1, reviewed at 27f7a3bf59244adaf41e4b5fa7e8d0826b4393e5 (fix commit
27f7a3bf5). No builds this round.

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 27f7a3bf5 |
| FND-002 | fixed | 27f7a3bf5 |

Round 2, reviewed at bb944486746544b9b4fe7e09318e19b6eb81091c (fix range
27f7a3bf5..bb9444867; pre-merge `make ci` exit 0 per the coordinator's log).
No builds this round. No new findings.

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-003 | fixed | bb9444867 |
