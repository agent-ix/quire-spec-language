---
id: SR-778
title: "QSL-303 code review of PR 515 (M-6d SEAM-3 handoff deletions)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; src/protocol_artifact/{checked_handoff,checked_predicate,content_identity,temporal_subject}.rs (deleted); src/protocol_artifact/native_temporal/* (deleted); src/protocol_artifact/mod.rs; tests/it/compiled_protocol_v2.rs; tests/it/main.rs; tests/it/native_temporal_owner.rs (deleted); tests/support/content_identity.rs (deleted); tests/support/mod.rs; tools/arch-lint/canonical_encoder.rs; README.md; docs/checked-native-handoffs.md; docs/native-temporal-owner.md; schemas/checked-*-v1.schema.json; schemas/native-temporal-*-v{1,2}.schema.json"
review_set: subset
---
## Summary

Ticket: QSL-303. PR: quire-spec-language#515. It is a pure deletion
PR (+2/-8866), with a Rust-review lane folded in.

What I checked, measured at the PR head:

- **The deleted modules are dead in Rust.** Nothing in `src/`, `tests/`,
  `examples/`, `xtask/`, `tools/` or any workspace crate names
  `checked_predicate`, `temporal_subject`, `native_temporal`,
  `checked_handoff` or `content_identity`. `make ci` builds the default,
  all-features, `--no-default-features` and `--features handoff-writer`
  configurations, so no feature-gated caller survives either. The `sha2`,
  `jsonschema` and `quire_canonical` imports the deleted code used are still
  used elsewhere, so no dependency was orphaned.
- **The coder's "still live" claims hold.** B8:
  `src/protocol_artifact/handoff/writer.rs:1553,1557,1710` calls
  `native::admit`, `native::emit` and `native::admit_v2` under the
  `handoff-writer` feature (Cargo.toml:30, and the two `required-features`
  examples). B9 types: `src/temporal/formula.rs:10`, `src/temporal/mapping.rs:19`
  and `src/state/work.rs:4` use `protocol_artifact::wire`. The ProtocolClause
  checker: `src/linking/composed/scopes/values.rs:265` dispatches
  `DeclarationKind::Protocol(protocol) => self.protocol(protocol)` into
  `scopes/protocol.rs:33`. `DeclarationKind::Protocol` is matched in 16 `src`
  files across the parser, linking, checking and native emission. The split to
  QSL-316 is real work that remains, not an excuse.
- **The arch-lint exemption removals are correct.** Both removed entries named
  modules that no longer exist, so leaving them would make `evaluate` report
  them as stale ("no remaining match") on the real tree. The repointed
  `a_stale_exemption_fails` has the same shape as before. `runtime::construction`
  is a live single-function `NotAnIdentity` exemption (canonical_encoder.rs:203),
  just like the old `checked_handoff::build`. The test overwrites that file with
  a canonical-encoder call and no hash site, and still asserts the
  `(module, None)` stale arm next to the `Some(function)` arm. The coverage
  it gives is unchanged.
- **Tests.** The only deleted tests are those that exercised the deleted modules.
  The `TC-142`, `TC-132`, `FR-048-AC-1` and `FR-049-AC-9` tags in the diff are
  context lines, not removals. See SR-779 for the spec-side trace consequences.

The problems are what the deletion left behind outside `src/` and `tests/`.
My own `make ci` run exited 0, with 6507 `ok` lines and 0 FAILED or
panicked.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | README still documents the deleted public API. It says `protocol_artifact::checked_predicate` and `protocol_artifact::temporal_subject` "publish the FR-051 native-owner handoffs" and that `protocol_artifact::native_temporal::{request,result}` "publishes the FR-052 formula-wide owner boundary". It also links two contract docs that now describe a surface that does not exist. Remove those two README paragraphs and delete `docs/checked-native-handoffs.md` and `docs/native-temporal-owner.md`. | README.md:83-103; docs/checked-native-handoffs.md; docs/native-temporal-owner.md |
| FND-002 | medium | Six schema files are orphaned. Their only readers were the `SCHEMA_BYTES` `include_bytes!` constants in the deleted modules (checked_predicate.rs:10, temporal_subject.rs:13, native_temporal/request.rs:24, result.rs:18, v2.rs:28,34). Nothing in the repo reads them now. Delete them. | schemas/checked-predicate-v1.schema.json; schemas/checked-temporal-subject-v1.schema.json; schemas/native-temporal-request-v1.schema.json; schemas/native-temporal-request-v2.schema.json; schemas/native-temporal-result-v1.schema.json; schemas/native-temporal-result-v2.schema.json |
| FND-003 | low | A downstream crate still uses the deleted API. `agent-ix/quire-contract-ir` origin/main imports `protocol_artifact::checked_predicate`, `temporal_subject` and `native_temporal::{request,result}`, including `SCHEMA_BYTES` and `ValidatedRequest`, in src/predicate/{admission,definition,reader}.rs and src/temporal/{admission,correspondence,formula,join,reader,request,valuation}.rs. It depends on an older QSL revision, so nothing breaks today. Its next QSL dependency bump will fail to compile unless IR first drops its SEAM-3 predicate/temporal consumers. This does not block the merge (QSL does not wait on downstream repos). It needs an IR-side Linear ticket. | quire-contract-ir Cargo.toml:24,51; quire-contract-ir src/predicate/*, src/temporal/* |

## Verdict

The Rust deletion is correct and complete. FND-001 and FND-002 are leftover
dead artifacts from the same deletion and should be removed in this PR.
FND-003 is a follow-up ticket, not a blocker.

## Dispositions

Round 1.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | README.md's two paragraphs on `checked_predicate`/`temporal_subject`/`native_temporal` are removed; docs/checked-native-handoffs.md and docs/native-temporal-owner.md are deleted; `git grep` finds no link to either doc outside historical `reviews/` records |
| FND-002 | fixed | all six schema files are deleted (schemas/ now holds only native-linked-package-1, native-run-result-1, native-state-input-1); `git grep` finds no remaining reference in src, tests, tools, xtask, Makefile, Cargo.toml or .github |
| FND-003 | deferred | IR-308 ("Drop imports of checked_predicate/temporal_subject/native_temporal after QSL-303's SEAM-3 deletion", Backlog) owns the IR-side follow-up; QSL does not wait on downstream repos |
