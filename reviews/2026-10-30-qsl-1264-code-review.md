---
id: SR-1878001
title: "code review (with rust-review) of PR #1878 (QSL-1264): Semantic IR fixture digest purge"
type: SpecReview
analysis: code-review
date: 2026-10-30
scope: "agent-ix/quire-spec-language; PR #1878 git diff origin/main...f8d4247f4 (55 files)"
review_set: subset
---
# SR-1878001: code review of PR #1878 (QSL-1264)

## Summary

Ticket: QSL-1264. Head f8d4247f4, base main 5fd8e71df (branch is up to date with main). Rust-review lane folded in.

The PR bumps `agent-ix-semantic-ir` / `agent-ix-extraction-frontend` to filament-core-data 1530aceea (PLAT-1205). It deletes `source.digest`, `package.manifestDigest`, `package.lockDigest` and `constructs[].manifestDigest` from every Semantic IR fixture, example and test builder. It also deletes the `PLACEHOLDER`/`DIGEST` consts and the recompute helpers (`refresh_manifest_digest`, `refresh_digests`, `jcs_digest`) and the inline `ByteDigest`-of-types blocks in `business_meanings.rs` and `variant_type.rs`.

Checks run: `cargo check --workspace --all-targets --all-features` (locked build, `--jobs 2`) and `cargo fmt --all --check` (clean). No cargo test and no clippy, under the host-overload rule.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Lockfile now carries two quire-rs 0.50.3 builds. FCD 1530aceea pins quire-rs 5b4be36, which includes #555 (clause-set and symbol digests removed) and #557 (semantic_core version gate removed). qsl-source still pins da10e4c. On main both resolved to da10e4c. QSL's own extraction (qsl-source) and FCD's `lift` (qsl-semantics intake) now run different quire-rs engines in one binary. The qsl-source crate doc (lib.rs:12-13) still says quire-rs is built transitively through FCD, as if it were one crate. The side effect is a split, not a move to 0.50.3 (main was already 0.50.3). Fix: bump qsl-source's quire-rs rev to 5b4be36 in this PR. `embedded_semantic_core_version` and `CONTRACT_VERSION` still exist at that rev. | Cargo.lock:1700-1745; qsl-source/Cargo.toml:27; qsl-source/src/lib.rs:12-13 |
| FND-002 | low | The removal leaves `let bounded = json!({..}); bounded` as a needless let-and-return inside the `push({ .. })` block. Push the `json!` value directly. Clippy `let_and_return` is default-warn and `make ci` runs clippy with `-D warnings`; clippy was not run here, so whether it fires on a macro initializer is unverified. | qsl-semantics/tests/support/native_values.rs:270-283 |
| FND-003 | low | `tc213_retained_map` and `tc213_type_identities` now do `let document = document.clone();` with no mutation after it. The clone existed only to call `refresh_manifest_digest`. Drop it and use the `&Value` parameter. | qsl-semantics/tests/it/tc235_business_meanings.rs:132,195 |

## Verdict

Findings. FND-001 needs a decision and a fix before merge. FND-002 and FND-003 are cleanup left behind by the removal.

The purge itself is correct and complete within the diff:
- No leftover digest: a grep of the PR head for `manifestDigest|lockDigest|PLACEHOLDER|jcs_digest|refresh_*digest` and zero `sha256:` digests outside `reviews/` finds none, and the frozen `sha256:878fa019…` in `switch.semantic-ir.json` is gone.
- No replacement check, stored expected value or compat shim was added. The `canonicalization.digest: "sha256-jcs"` profile member is an algorithm name, not a recorded digest, so it stays.
- No real assertion was weakened. `tc235_shared_k2_fixture_preserves_meaning_contract` still asserts the 8 types, `types[1].kind == {module: BUSINESS, name: value}` and `types[1].target == ix://quire/native/Timestamp`. Only the two digest-equality lines were removed.
- The deleted `qsl_owned_digest_fixture_uses_sha256_jcs` tested only the deleted fixture helper. It covered no production behaviour.
- `refinement_versioning.rs`'s rewritten `replacen` pattern still matches the example (`"version": "1.0.0",\n    "mappingVersions"`).
- `cargo fmt --check` is clean. `cargo check --workspace --all-targets --all-features` passes (exit 0, no warnings).
