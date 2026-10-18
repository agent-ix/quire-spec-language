---
id: SR-12094
title: "Code review of quire-spec-language PR #943: cut the native couplings (QSL-498 A0)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@5932793c77bcd8e0fe4a2c19557ce354191de0b3; PR #943: src/command.rs, src/command/wire.rs, src/command/output.rs, src/command/output/types.rs, src/protocol_artifact/{domain,mod,models}.rs, src/protocol_artifact/native/types.rs, qsl-semantics/src/model/domain_package.rs, examples/config-version/{cases,fixtures,spine_corpus}.rs, examples/config_version_fixtures.rs, tests/support/mod.rs, tests/it/{config_version_spine,domain_protocol_emission,formatter_identity}.rs"
review_set: subset
---
# Code review of quire-spec-language PR #943

## Summary

Ticket: QSL-832 (slice A0 of QSL-498). Code review with the rust-review lane
folded in. Read-only review of `git diff origin/main...HEAD`; nothing was
built, so compile and lint problems were hunted by reading.

Checked and clean:

- `RunCause::Input` removal is dead. Nothing in `src/command*`, `src/main.rs`
  or `src/cli.rs` names `InputReadError`, `RuntimeReference` or a runtime
  reader; the only `From` impls into `RunCause` are `ir::Diagnostic` and
  `ExtractionMode`; `Edition::Native` already refuses "native run is
  retired". `RunCause::code`, `output::error` and `types::Stage`/`Details`
  lose their `Input` arms together; no test, schema or spec names stage
  `input`. Exit codes are unchanged.
- `IgnoredAny` change: `request.selection/snapshots/invocations` are only
  read with `is_some()` (`command.rs:992-1006`, `clause.rs:38-40`) and
  `Vec::len` for intake file counts (`command.rs:943,951`). `from_object` is
  generic over `T: Deserialize<'de>`, so `IgnoredAny` still requires a JSON
  object. `Selection` dropping `Serialize` is safe: `Request` is not
  serialized. TC-450 step 4 (`spine_run.rs:355`) pins the refusal
  (invalid-request, stage request, exit 20) for selection, snapshots and
  invocations.
- Consumer audit by signature: `DOMAIN_PACKAGE_PROFILE` has four importers,
  all moved to `qsl_semantics::model::domain_package`; the intra-doc link at
  `command.rs:629` resolves through the new `use`. `support::config_version`
  callers (`config_version.rs`, `config_version_refusals.rs`,
  `state_scalar_lowering.rs`) use only `Case`, `MODEL`, `model`, `write`,
  all still exported. `spine.rs`'s `use super::cases::...` resolves to
  `spine_corpus::cases`. xtask and qsl-package include only `spine_unit.rs`.
- `duplicate_mod`: `spine_corpus.rs`, `cases.rs`, `spine.rs` and
  `spine_unit.rs` are each loaded once per crate (`it` binary and the
  `config_version_fixtures` example).
- Import graph: qsl-semantics gains a `const` and no `use`; the root crate
  already depends on qsl-semantics. arch-lint `canonical_encoder` has no rule
  touching the changed modules.
- Test count in `config_version_spine.rs` is 21 before and after.

## Verdict

Changes requested. One high finding: `spine::write` never creates its
directory, and the rewritten `build()` and determinism test now call it on
directories nothing created, so most TC-469 tests panic. One medium finding:
`fixtures::CASES` has no user left in the `it` binary, which fails clippy
under `-D warnings`.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | `spine::write` writes with bare `std::fs::write(directory.join(file), ..)` and never creates `directory`; the old `build()` relied on `fixtures::write`'s `create_dir_all` (`fixtures.rs:380`). Now `build()` calls `spine::write` directly, and every caller except `run_case` passes a fresh `tempdir().join(<name>)` that does not exist (lines 245, 287, 341, 369, 387, 405, 431, 458, 494, 534, 607, 639, 661, 702, 728, 836, 867, 1203, 1273), and the determinism test writes into `first/<case>` and `second/<case>` under a root that only holds `tempdir`. Each one panics with NotFound at `expect("spine::write emits ...")`. The doc on `spine::write` ("beside the native ones `fixtures::write` already wrote") is now false for the spine path. Fix: `std::fs::create_dir_all(directory)?` at the top of `spine::write`, and update its doc. | examples/config-version/spine.rs:177-178; examples/config-version/spine.rs:299-303; tests/it/config_version_spine.rs:195-197; tests/it/config_version_spine.rs:1013-1015 |
| FND-002 | medium | `pub const CASES: &[Case] = cases::CASES;` in `fixtures.rs` has no user left in the `it` test binary: every former `config_version::CASES` caller now uses `config_version_spine::cases::CASES`, and `config_version.rs`, `config_version_refusals.rs` and `state_scalar_lowering.rs` never used it. `support` is `pub(crate)` in a test binary, so rustc reports `constant CASES is never used`, and `make ci`'s `cargo clippy --workspace --all-targets -- -D warnings` fails. Fix: delete `fixtures::CASES` and have `examples/config_version_fixtures.rs` iterate `config_version_spine::cases::CASES`. | examples/config-version/fixtures.rs:25-26; examples/config_version_fixtures.rs:23 |

The finding table holds defects only. The checks that were verified and found
clean are listed under Summary.

## Dispositions

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | `spine::write` creates `directory` first; its doc no longer refers to the native files. |
| FND-002 | fixed | `fixtures::CASES` deleted; `examples/config_version_fixtures.rs` iterates `config_version_spine::cases::CASES`. Every other `fixtures.rs` item (`MODEL`, `Case`, `model`, `write`) still has a user in the `it` binary. |
