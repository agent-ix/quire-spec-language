---
id: SR-12340
title: "Code review of QSL#1232 and SV#60 (IR-585: WrongSnapshotCause moves to quire-semantic-value::call)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@509a7919a0dddd42ac070b76be7f08c726cea39f with agent-ix/quire-semantic-value@1cf30d9c33006cf8d9cf017973dbf10550839d43; QSL qsl-eval/src/value/expression/causes.rs, qsl-eval/tests/it/{model_reference_queries,tc235_business_meaning_runtime,type_invariants}.rs, qsl-semantics/src/check/{mod,refusal}.rs, qsl-semantics/src/check/check/typing.rs, qsl-semantics/src/check/lowering/tests/expression_depth.rs, qsl-semantics/tests/it/{operation_clauses,state_clauses}.rs, xtask/src/{definition_scan,seam_probe}.rs; SV Cargo.toml, src/call.rs"
review_set: subset
---

## Summary

Ticket: IR-585. One review covering both halves of the S1 move: SV#60
(adds `WrongSnapshotCause` with `as_str` and its `#[cfg(seam_probe)]`
variant to `quire_semantic_value::call`, and declares `cfg(seam_probe)` in
`[lints.rust] unexpected_cfgs`) and QSL#1232 (deletes the type from
`qsl_semantics::check::refusal`, drops it from `check`'s `pub use` list and
repoints every caller). The rust-review lane is folded in.

Measured:

- **SV gate.** `make ci` at 1cf30d9 (fmt-check, clippy `-D warnings` incl.
  `thumbv7em-none-eabi`, test, `build-no-std --locked`, deny, docs) exit 0.
  `RUSTFLAGS="--cfg seam_probe" cargo clippy --all-targets -- -D warnings`
  exit 0, so the probe variant and its `as_str` arm compile clean.
- **`unexpected_cfgs`.** Correct and load-bearing: SV's lint lane is
  `-D warnings` and `#[cfg(seam_probe)]` would otherwise raise
  `unexpected_cfgs`. Same form as QSL's own crates (`Cargo.toml:168`).
- **Seam probe.** QSL worktree with an uncommitted `.cargo/config.toml`
  `[patch]` pointing SV at the #60 worktree: `cargo run -p xtask --
  seam-probe` exit 0, "17 checked-in locations confirmed", including
  `qsl-eval/src/value/expression/causes.rs::ProtocolClauseSnapshot::catalog_code`.
  `RUSTFLAGS` reaches the git dependency, so the QSL match over the SV type
  is still made non-exhaustive by the SV probe variant, and the
  `seam_probe_eval_downstream` arm still lets the crates above `qsl-eval`
  compile. FR-401's "receiving crate declares the `seam_probe` cfg" holds.
- **No re-export, no stray callers.** `grep -rn WrongSnapshotCause --include=*.rs`
  over the QSL tree: every reference is `quire_semantic_value::call::WrongSnapshotCause`
  or an unqualified name under that `use`. `refusal.rs`'s import is a private
  `use` in a private `mod refusal`; `check/mod.rs`'s `pub use refusal::{..}`
  no longer lists it. No `pub use` of it anywhere in QSL (ADR-011 §7.2).
- **Lock and order.** QSL `Cargo.lock` still pins SV `ac0e1379` (no
  `WrongSnapshotCause`), so #1232 builds only once SV#60 is merged and
  `cargo update -p quire-semantic-value` is committed on #1232. That is the
  order both PR bodies state, and it is right. SV `ac0e137..1a38c06` (main,
  which #60 sits on) is additive only (`Conversion::identified_quantity`,
  doc edits), and the patched build compiled every QSL crate against it.
- **Focused QSL checks** (patched), all exit 0: `cargo clippy -p
  qsl-semantics -p qsl-eval -p xtask --all-targets --all-features -- -D
  warnings`; xtask `definition_scan` and `closed_enums_*` tests (7 passed);
  qsl-semantics `it` operation/state clauses (132 passed) and lib
  `expression_depth` (3 passed); qsl-eval filtered tests (95 + 7 passed);
  qsl-semantics doctests (23 passed). No full `make ci` was run: it needs
  the lock bump first.
- Definition and semantics: the SV enum is variant-for-variant the deleted
  QSL enum (same derives, no `#[non_exhaustive]`, same `as_str` tags).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | `closed_enums_carry_no_non_exhaustive_attribute` drops `WrongSnapshotCause`, so TC-161 step 10's expected result (none of the five types carries `#[non_exhaustive]`) is no longer checked for it, while the test's doc comment (`seam_probe.rs:1548-1552`) still says it is. The check is still possible: read the SV source from the cargo checkout as `canonical_types` already does for the leaves, or amend TC-161 step 10 and the doc to four types. Low because `xtask seam-probe` would report `ProtocolClauseSnapshot::catalog_code` missing if a forced `_` arm ever replaced the seam | xtask/src/seam_probe.rs:1548-1567 |
| FND-002 | low | Stale comment: `FamilyKind::catalog_code_prefix`'s downstream-arm comment still lists `WrongSnapshotCause` among this crate's (qsl-semantics') probe variants | qsl-semantics/src/family/mod.rs:140-143 |
| FND-003 | low | SV's new test `each_wrong_snapshot_cause_names_its_tag` carries no `#[trace(..)]` in a repo that traces tests; SV has no criterion for the `call` module, so the tag would point at QSL's FR-090-AC-6 / FR-401-AC-3 or the test stays an untraced unit test by decision | quire-semantic-value src/call.rs:186-193 |

## Verdict

Code is correct. The move is exact, there is no re-export at the old path,
no caller is left on the old path, and the seam probe still sees the QSL seam
over the moved type with the probe variant in SV. The `unexpected_cfgs`
addition is correct. The merge order (SV#60, then a `cargo update -p
quire-semantic-value` commit on #1232) is right and is required: #1232 does
not compile against its own lockfile until then. Findings are low only. See
the gap-analysis artifact (SR-12341) for the two medium traceability gaps.
