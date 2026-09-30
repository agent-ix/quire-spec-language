---
id: SR-683
title: "QSL-271 base review of the FR-110 pinned root headers in FR-108 and TC-452"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language; spec/functional/FR-108-run-the-configversion-spine-corpus.md; spec/test-cases/TC-452-spine-run-entry-lives-in-qsl-replay.md; spec/functional/FR-100 to FR-110; spec/test-cases/TC-450 to TC-469; code evidence qsl-replay/src/spine/call/tests.rs, qsl-semantics/src/value/definition.rs, qsl-foundation/src/source.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-108
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-452
    type: reviews
---

## Summary

Ticket: QSL-271 (PR agent-ix/quire-spec-language#481, spec only, 6 changed
lines, based on main). Each value was recomputed at the PR head,
not taken from the PR body.

Measured and clean:

- `FIXTURE_F` (qsl-replay/src/spine/call/tests.rs:299), parsed from its Rust
  literal, is 237 bytes with SHA-256
  `3cb8ab70e4d3187dae8621768491c4d2eb0c8c0b82d330d4f2fba72883f6e77c`. The
  literal `5` is byte 233 (half-open end 234), line 3, columns 54 to 55,
  under `Position`'s zero-based byte and one-based line and Unicode scalar
  column (qsl-foundation/src/source.rs:80-89). The test's own asserts agree
  (tests.rs:325, 418, 420).
- TC-452 step 4's fenced fixture, with its three-space indent removed, equals
  `FIXTURE_F` byte for byte.
- The new header matches the `Root` row of `DefinitionLock::pinned()`'s
  catalog (qsl-semantics/src/value/definition.rs:347-355): identity
  `quire.value.complete/v1`, revision `1-draft.2`, digest `c8c7ae9f…25b16`.
  FR-110 (lines 74-87) resolves a header profile only against that row.
- No profile header in FR-100 to FR-110 or TC-450 to TC-469 still uses the
  placeholder, apart from FR-110:173, which quotes it as history. The
  remaining `version "1"` spellings (FR-108:52, TC-456:22) are `model`
  declarations of the domain package `example/config-version` version `1`
  (FR-108:40-41). FR-110 resolves only `profile` headers, so they are
  correctly left alone.
- `quire validate` over the two changed files exits 0.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | FR-110's Status says FR-108:51 and "TC-452 step 1's source (TC-452:40, and its byte count) spell `version "1"`" and need the root revision "before FR-110 lands". This PR makes that false, and the fixture is in step 4, not step 1. Failure scenario: a reader of FR-110 believes the A05 dependency is still open and re-does or blocks on it. Fix: reword the bullet to record that QSL-271 (#481) moved both rows. | spec/functional/FR-110-resolve-header-profile-selections-at-e3.md:186-189 |

## Verdict

Approve. Every recomputed value matches; FND-001 is a non-blocking stale
note outside the diff.
