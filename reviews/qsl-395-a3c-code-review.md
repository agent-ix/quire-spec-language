---
id: SR-1292
title: "Code review of quire-spec-language A3c: recomputed identities refuse content-mismatch"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@b3a2a6e3261429efffc7573d5c94ecfdb2146b09; PR #629, git diff origin/main...HEAD: qsl-foundation/src/digest.rs, qsl-package/src/checked_v2/tests.rs, qsl-replay/src/execute.rs, qsl-replay/src/request.rs, qsl-replay/src/spine/clause/tests.rs and tests/{frame,frame_replay,state_clause_replay}.rs, qsl-semantics/src/library/{mod,binding_tests}.rs, qsl-semantics/src/model/{intake,observation,refusal}.rs, qsl-semantics/tests/it/state_clauses.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-056
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-071
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-106
    type: reviews
---
# Code review of quire-spec-language A3c

## Summary

Ticket: QSL-395 (A3c, plan row MS-03). PR agent-ix/quire-spec-language#629,
head b3a2a6e32. The Rust lane (rust-review) is folded into this file.

Ruling applied, not raised (owner, matches QSpec FR-272):
`byte-digest-mismatch` is a raw-bytes digest compared with its selection or
declared reference; `content-mismatch` is a selected or pinned content
identity compared with the one recomputed from the subject, carrying both.

What the change does, checked against the code:

- Intake check 3 (`check_package_digest`) now returns a two-arm
  `DigestMismatch`: `Content { recomputed }` when the bytes parsed,
  `RawBytes { digest }` when they did not. `admit` maps them to
  `ModelRefusalCause::ContentMismatch { selected, recomputed }` (detail names
  both hex digests) and the kept `ByteDigestMismatch { expected, actual }`.
- Observation check 1.3 maps the same split; the `content-mismatch` record
  carries `selected` and `recomputed` fields.
- `ReplayRequest::decode`: a mismatch under `sha256-jcs` refuses the new
  `ContentMismatch { selected, recomputed }`; raw domains keep
  `ByteDigestMismatch`. A `sha256-jcs` entry that does not parse refuses
  `NotAPackageDocument` first, so the raw arm is never reached by a jcs entry.
  The split is by digest domain, with no alias variant.
- `LibraryCause::ByteDigestMismatch` is renamed `ContentMismatch`
  (`"content-mismatch"`), no alias; `PinMismatch { pinned, presented }` already
  carries both values.
- `DependencyIdentityMismatch` changes only in its doc comment.

Remaining `byte-digest-mismatch` producers, each a raw-bytes compare:
reader-refused domain-package bytes (intake, observation), raw-domain
byte-provision entries (request.rs), and qsl-cst's source digest. Nothing that
recomputes a canonical digest or a `package_id` still spells
`byte-digest-mismatch` in code.

Rust idioms: the `fn([u8; 32]) -> DigestMismatch` pointer in
`check_package_digest` is a slightly roundabout way to write a two-arm match,
but correct; not raised. No new `unwrap`/`expect` in production paths, no
integer conversions, no new public surface beyond the two enum variants.

Focused tests run at b3a2a6e32: see Verdict.

## Verdict

Approve with low findings. The code follows the ruling at every site the
brief names. `make ci` exit 0 at this head (coder's log). Focused tests run by the
reviewer through locked-build.sh, all passing: qsl-semantics lib (16:
intake and observation content-mismatch, observation digest_tests, library
binding_tests), qsl-replay lib (6: tc_186 x2, edited_invocation_bytes x2,
run_clause snapshot edit, frame stale_or_mismatched), qsl-semantics `it` (2:
TC-465 rows 4 and 29). The test-oracle gap
on the replay-request variant is recorded in the gap analysis (SR-1293
FND-001), not repeated here.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | `DependencyIdentityMismatch` (and `PackageIdMismatch`) render `stale_dependency: ...` with no cause. ADR-015 rule 7 now names `content-mismatch`, but that cause exists only in a doc comment; the request-side `ContentMismatch` renders `stale_dependency/content-mismatch:`. Spell the cause in the `#[error]` string so the refusal names it. | qsl-replay/src/execute.rs:116-127 |
| FND-002 | low | The TC-147 test's doc comment still cites `refuses_byte_digest_mismatch`, which this PR renamed `a_parsed_document_under_another_digest_refuses_content_mismatch`. | qsl-semantics/tests/it/model_intake.rs:822 |
| FND-003 | low | `run_clause_refuses_a_snapshot_edited_after_its_digest_was_taken` is traced to FR-109-AC-5 / TC-468 step 5, but FR-109-AC-5 is report determinism and the FR-100 outcome mapping; nothing in it is a digest check. This PR edited the test's oracle. The behaviour is FR-106-AC-3's (TC-465). Retag it. | qsl-replay/src/spine/clause/tests.rs:2196-2202 |
