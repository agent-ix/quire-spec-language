---
id: SR-1294
title: "Spec review of quire-spec-language A3c: recomputed identities refuse content-mismatch"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language@b3a2a6e3261429efffc7573d5c94ecfdb2146b09; PR #629, git diff origin/main...HEAD: ADR-015; FR-056, FR-071, FR-073, FR-087, FR-106, FR-108, FR-116, FR-122, FR-261; TC-145, TC-147, TC-465, TC-491, TC-515, TC-517, TC-733; US-010; plus the text those edits must agree with: ADR-013:937, FR-098, FR-111-AC-2, TC-186, TC-211, TC-444, FR-260, and quire-specification PR #187 (97cc67d: FR-272, native-diagnostics, FR-322, FR-323, TC-277)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-186
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-491
    type: reviews
---
# Spec review of quire-spec-language A3c

## Summary

Ticket: QSL-395 (A3c, MS-03). PR #629 at b3a2a6e32, read beside
quire-specification #187 at 97cc67d.

Ruling applied: `byte-digest-mismatch` = a raw-bytes digest compared with its
selection or declared reference; `content-mismatch` = a selected or pinned
content identity (`sha256-jcs` digest or `package_id`) compared with the one
recomputed from the subject, carrying both.

Consistent with the ruling, checked line by line: ADR-015 rule 7; FR-056
check 3 body, AC-2 and AC-12 (the reader-refused raw case keeps
`byte-digest-mismatch`); FR-056:93-104, :164 and FR-260:40 (raw cases,
unchanged and correct); FR-071 body and AC-6 (split by domain); FR-073;
FR-087-AC-11; FR-106 check 1.3 and AC-7; FR-108; FR-116-AC-4; FR-122-AC-5;
FR-261-AC-2; TC-145, TC-147, TC-465 rows 4 and 29, TC-515, TC-517, TC-733;
US-010; TC-211 and TC-444 step 3 (both source-bytes entries, correctly
`byte-digest-mismatch`). The envelope node-identity uses in ADR-014, ADR-017,
ADR-025 already said `content-mismatch` and fit the ruling.

QSpec #187 agrees on FR-272, native-diagnostics, FR-322:103, FR-322-AC-43,
FR-323 rules 4 and 5, FR-323-AC-7, TC-277 BP-05, BP-07, BP-11. The
disagreements are FND-001 to FND-003.

## Verdict

Changes requested on the spec side: five medium findings, all in text this
PR's sweep should have reached (ADR-013:937 is the line the brief names; TC-186
and TC-491 are TCs of changed ACs). The code is right at each site; the text
around it is not yet.

Recommended single spelling for TC-277 BP-02 and ADR-013:937's absent input:
`missing_declaration`, which is what QSL's decode emits
(`IncompleteByteProvision`, request.rs:358-360, :440) and what TC-186 step 5
tests. Neither digest cause fits: an absent entry has no bytes and so no
recomputed or raw digest, and QSpec native-diagnostics itself says "A missing
artifact cannot supply an invented actual digest." QSpec's
`byte-digest-mismatch` and QSL's `content-mismatch` are both wrong for BP-02.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | ADR-013 C-13's validation row says an input absent from the byte provision, or whose bytes do not match their digest, refuses `stale_dependency`/`content-mismatch`. The code refuses an absent input `missing_declaration` (`IncompleteByteProvision`), a raw-domain mismatch `byte-digest-mismatch`, and only a `sha256-jcs` entry `content-mismatch`. Rewrite the row with the three cases. | spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md:937; qsl-replay/src/request.rs:338-360, :577-590 |
| FND-002 | medium | Cross-repo: QSpec #187 TC-277 BP-02 (an input missing from `byte_provision`) keeps `stale_dependency`/`byte-digest-mismatch`, QSL ADR-013:937 says `content-mismatch`, and the QSL code emits `missing_declaration`. Three spellings for one refusal. Use `missing_declaration` in both repos (BP-02, ADR-013:937, and FR-071-AC-5, which names no cause). Separately, BP-03 says `byte-digest-mismatch` for any mismatched entry; under this PR's split a `sha256-jcs` entry refuses `content-mismatch`. Qualify BP-03 to a raw-domain entry and add the `sha256-jcs` case. | quire-specification spec/test-cases/TC-277-replay-byte-provision.md:39-40 (PR #187); spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md:937; spec/functional/FR-071-implement-typed-replay-request.md:136 |
| FND-003 | medium | Cross-repo: QSpec #187 now says the proved package's recompiled `package_id` differing from the request's refuses `stale_dependency`/`content-mismatch` "(QSL's `PackageIdMismatch`)" (FR-323 rule 5, TC-277 BP-05). QSL names no cause for `PackageIdMismatch` anywhere: FR-098 says "a stale `package_id`", ADR-015:207 says "the existing `PackageIdMismatch`", ADR-013:937 "stale `package_id`". It is a declared `package_id` against a recomputed one, so the ruling makes it `content-mismatch`. State it in FR-098 and ADR-013:937 (and in the `#[error]` string; SR-1292 FND-001). | spec/functional/FR-098-execute-a-replay-request.md:130, :150; spec/decisions/ADR-015-compile-and-replay-against-dependencies.md:207; quire-specification spec/objects/interfaces/FR-323-native-runtime-envelope.md:63-65 (PR #187) |
| FND-004 | medium | TC-186 was not updated, though the PR body lists it. Step 3 builds an entry whose digest "does not match the RFC 8785/JCS bytes it is claimed to address" and Expected Results says `byte-digest-mismatch`. Under FR-071-AC-6 as amended, a JCS entry refuses `content-mismatch`; the test's step 3 actually uses a source-bytes entry. Make step 3 a raw-domain entry, and add the `sha256-jcs` step that the test already runs (`stale_package`), expecting `content-mismatch` carrying both digests. | spec/test-cases/TC-186-replay-request-digest-only-byte-provision.md:43-47, :75-77; qsl-replay/src/request.rs:940-967 |
| FND-005 | medium | TC-491 step 2 still links against a catalog that holds a root's identity "with other bytes" and expects `stale_dependency` (now `content-mismatch`). FR-111-AC-2 says a root whose identity the catalog holds "resolves whatever revision label or bytes the catalog's definition carries", and the bundle code has no stale-dependency refusal. The PR renamed a refusal that no longer exists. Delete the "hold it with other bytes" clause and its expected cause. | spec/test-cases/TC-491-link-a-complete-v1-definition-bundle.md:28-30, :49-51; spec/functional/FR-111-*.md:137 |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-006 | medium | Cross-repo, QSpec #187 at a0027709: FR-272, FR-323-AC-5 and TC-277 BP-03 now require `byte-digest-mismatch` to name the declared digest and the digest of the supplied bytes. QSL does not carry the actual digest at two raw-byte sites. `ReplayRequestRefusal::ByteDigestMismatch(String)` holds only the declared digest, and observation check 1.3's raw arm emits a `byte-digest-mismatch` record with no fields. Intake's `ByteDigestMismatch { expected, actual }` already carries both. Carry the raw digest in the request variant and the observation record, and say so in FR-071-AC-6 and FR-106 check 1.3. | qsl-replay/src/request.rs:343-344, :593; qsl-semantics/src/model/observation.rs:581-584; quire-specification spec/objects/interfaces/FR-323-native-runtime-envelope.md (AC-5), spec/test-cases/TC-277-replay-byte-provision.md:40 |

## Dispositions

Round 1, reviewed at 27f7a3bf59244adaf41e4b5fa7e8d0826b4393e5 (fix commit
27f7a3bf5), with QSpec #187 at a00277090362f29cab250280be9782510b219f5d.
FND-002 is settled by the coordinator's ruling: `missing_import`/`missing-selection`
in both repos, per FR-271's "requested typed dependency". That supersedes
this review's `missing_declaration` recommendation. Both repos now say it:
QSL ADR-013:937, FR-071-AC-5, TC-186 step 5, TC-444, and the code; QSpec
FR-323 and AC-5, and TC-277 BP-02. BP-03 is now limited to raw bytes, and
the new BP-13 is the `sha256-jcs` case, which matches QSL's split. FR-323-AC-9
and BP-05 match QSL's `PackageIdMismatch` spelling.

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 27f7a3bf5 |
| FND-002 | fixed | 27f7a3bf5 (QSL); QSpec a0027709 |
| FND-003 | fixed | 27f7a3bf5 |
| FND-004 | fixed | 27f7a3bf5 |
| FND-005 | fixed | 27f7a3bf5 |

Round 2, reviewed at bb944486746544b9b4fe7e09318e19b6eb81091c (fix range
27f7a3bf5..bb9444867). The coordinator's ruling: QSL carries both the
declared digest and the raw digest. `ReplayRequestRefusal::ByteDigestMismatch
{ declared, actual }` and observation's raw-arm record (`selected`, `actual`)
now do, and FR-071 body, FR-071-AC-6 and FR-106 check 1.3 say so, which
matches QSpec #187 FR-272, FR-323-AC-5 and TC-277 BP-03. No new findings.

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-006 | fixed | bb9444867 |
