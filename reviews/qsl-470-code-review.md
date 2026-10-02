---
id: SR-1203
title: "Code review of quire-spec-language PR #592: header profiles and bundle roots resolve by identity"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@d1ae0de9c9a333984de53443350ddd214aec2387; PR #592 diff against origin/main (72 files): qsl-foundation/src/selection.rs, qsl-cst/src/{parser,grammar,diagnostic}.rs, qsl-semantics/src/check/profile.rs, qsl-semantics/src/library/{bundle,bundle_tests}.rs, qsl-semantics/src/qspec_diagnostics.rs, qsl-eval/src/simulation/*, src/complete/editor.rs, src/state/*, src/temporal*, qsl-replay/src/spine.rs and test/fixture updates, tests/fixtures/parser-differential/baseline.txt, Makefile"
review_set: subset
---
# Code review of quire-spec-language PR #592

## Summary

Ticket: QSL-470. The PR changes a header profile to `profile <alias> = "<identity>";`
and makes E3 resolve it by identity alone. `DefinitionRef` becomes
`{authority, identity}`, `DefinitionCatalog` is keyed by identity, and
`link_bundle` resolves by identity. It deletes every stale-profile and stale-root
cause, and it deletes the observation-contract, FR-095 support-table and sampler
revision pins.

What the coordinator asked to check:
- The parser-differential baseline was re-recorded honestly. All 200 changed
  lines are in the `complete` family, which is the only family whose generator
  header changed (`COMPLETE_HEADER` drops `version`/`digest`). The
  `historical` and `composed` buckets are unchanged. The whole-baseline test
  (`both_parsers_match_the_whole_recorded_baseline`, run with
  `--include-ignored`) passes against the current parsers.
- The fixture F updates are honest. I recomputed them independently. The new
  `FIXTURE_F` is 136 bytes and its SHA-256 is
  `0c84cc4a...eca36eb`, which matches the test. The removed
  ` version "1-draft.2" digest "sha256:<64 hex>"` is 101 bytes, so byte
  233 to 234 moves to 132 to 133, while line 3 and columns 54 to 55 stay the
  same. Deleting the `len() == 237` assertion is correct, because a fixture
  byte count is not behaviour.
- Rust lane (rust-review): there is no new panic on a production path.
  `qspec_diagnostics` is `#[cfg(test)]` and its panics are test failures.
  `ProfileCatalog::new` now checks its limit while inserting, so it bounds an
  iterator input before collecting all of it. No compatibility path is left:
  the old header spelling is a syntax error.
- Conformance: both new tests ran against the local QSpec checkout
  (`QSPEC_DIR=~/dev/quire-specification`): 12 bundle pairs and 2 profile
  pairs are listed. QSpec is read in place, not copied.
- Gates run at this head with a worktree-local target dir: `cargo test` on
  qsl-foundation, qsl-cst, qsl-semantics, qsl-eval and qsl-replay passed, and
  the root `it` tests for parser_differential, complete_editor,
  composed_temporal and native_boundaries passed.

Rulings applied and not raised: the DefinitionLock members, the
quire-specification dependency, the lock bytes and the catalog byte read
(A1b), the `depth` cause (B4), FR-110-AC-7/8, and `QSPEC_DIR`.

## Verdict

Changes requested: one low cleanup finding.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | Revision and digest wording, plus one cause mapping, are left behind by the deletions. (1) `Refusal::AuthorityRevision` no longer has anything to do with a revision; its doc now says "identities or digests are malformed". Rename it. (2) `CompleteCause::ByteDigestMismatch::is_cause_of` still admits `StaleDependency`, but its only producer is now `SourceReadCause::DigestMismatch` (`source_digest_mismatch`), so the closed cause relation admits a pair nothing emits. Narrow it to `SourceDigestMismatch`. (3) Stale comments: the `NotSimulated::GeneratorMismatch` doc still says "`1-draft.1`"; the `FIXTURE_F` doc still says "237 bytes" and "byte 233 to 234"; `value-format.native` line 2 still says "the profile digest is a placeholder". | src/state/input.rs:466; qsl-cst/src/diagnostic.rs:166-169; qsl-eval/src/simulation/not_simulated.rs:34-35; qsl-replay/src/spine/call/tests.rs:293-297; tests/fixtures/value-format.native:2 |
