---
id: SR-15600
title: "Spec review of quire-spec-language PR #1236: STD-301, the audit-unsafe gate for the evaluator closure"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@4f678f4f684c215d52f72576dbdb5124386eecaa; PR #1236 (git diff origin/main...4f678f4f6): spec/non-functional/NFR-016-audit-unsafe-code-in-the-evaluator-closure.md, spec/test-cases/TC-1130-*.md, spec/test-cases/TC-1131-*.md, spec/assurance/AD-001-shared-evaluation-core-interface.md, spec/spec.md; read for context: FR-400, FR-401, FR-403, NFR-015, ADR-011 FB-05 and layer table, agent-ix/quire-eval main 57712bd93 (Cargo.toml, Makefile, deny.toml, Cargo.lock, crates/quire-checked), quire-canonical, quire-exact, quire-walk, quire-semantic-value manifests"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/NFR-016
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-1130
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-1131
    type: reviews
  - target: ix://agent-ix/quire-spec-language/AD-001
    type: reviews
---
# SR-15600: Spec review of quire-spec-language PR #1236: STD-301

## Summary

Ticket: STD-301 (S8, IR-592). Sub-analyses folded in: integrity, EARS, dependency, failure-domain, scope-boundary. `make check-spec-validation` passes at the head with no diagnostic on the new or edited files. NFR-016, TC-1130 and TC-1131 are unique across all open PRs, including #1237 (STD-302), and that PR's AD-001 hunks do not overlap this one's. main 48108c9c1 is the head's parent.

Evidence: I ran cargo-geiger 0.13.0 over `agent-ix/quire-eval` main 57712bd93 with `--manifest-path <abs>/crates/quire-checked/Cargo.toml --no-default-features --target thumbv7em-none-eabi --output-format Json --locked`. The closure is the same as quire-eval's would be, minus quire-eval itself. Results:

- **Host build.** Every artifact landed in `target/debug`, so geiger built for the host even with `--target`. Its package graph still drops `cpufeatures`, a dependency only on host architectures.
- **Report shape.** The JSON report has the keys `packages`, `packages_without_metrics` and `used_but_not_scanned_files`. Each package has used and unused counts and `forbids_unsafe`.
- **Unsafe in the closure.** Third-party crates with unsafe code: allocator-api2, block-buffer, generic-array, num-bigint, once_cell, ryu-js, serde (unused only), serde_core, sha2 and unicode-normalization. Proc-macro crates and the crates under them also appear in the report.
- **Command line.** `cargo geiger -p quire-checked` at the workspace root refused the virtual manifest. A relative `--manifest-path` was refused too.
- **Feature unification.** In a scratch copy with one extra workspace member that enables `quire-canonical/serde_json`, `cargo metadata --filter-platform thumbv7em-none-eabi` puts serde_json, itoa, memchr and zmij into quire-checked's normal closure. `cargo tree -p quire-checked --no-default-features -e normal,no-proc-macro --target thumbv7em-none-eabi` gives the per-package set, without them.

## Verdict

**NOT MERGEABLE.** The design is sound: the gate lives in quire-eval, the inventory is keyed by name only, stale entries fail, and proc-macro crates are excluded. As written, though, the gate fails on today's closure (FND-001), its literal command does not run (FND-002), and its closure source disagrees with what the scan covers (FND-003). FND-004 and FND-005 need decisions or test changes.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | [major] The first-party rule counts unused source files, so it fails on the real closure today. quire-canonical's integration tests (tests/memory.rs, tests/drop_value.rs: a counting allocator) carry unsafe code. geiger reports 37 unsafe expressions, 2 unsafe impls and 6 unsafe methods as unused for quire-canonical, so `first-party-unsafe quire-canonical` fires and NFR-016-AC-1 cannot pass. geiger aggregates counts per package, not per file, so the checker cannot exclude test, bench or example files. Owner decision: count only the library target's files for first-party crates, keep counting everything for third-party crates, or require the leaf to move its test allocator out. | spec/non-functional/NFR-016-audit-unsafe-code-in-the-evaluator-closure.md:77-84,86-94 |
| FND-002 | high | [major] The literal command `cargo geiger -p quire-eval --target thumbv7em-none-eabi --output-format Json --locked` fails at the workspace root on cargo-geiger 0.13.0: "is a virtual manifest, but this command requires running against an actual package". quire-eval's root Cargo.toml is a virtual manifest. An absolute `--manifest-path` to the crate's manifest works. The Gate section, AC-1 and TC-1130 step 3 all fix the failing form. Specify the scan by its inputs (package, target, report format), or use the absolute `--manifest-path` form. | NFR-016.md:126-128,192; TC-1130.md:21-22 |
| FND-003 | medium | [major] The checker's closure comes from `cargo metadata`, which resolves features for the whole workspace. The scan is `-p quire-eval`. Once quire-eval-corpus (deps: quire-canonical, serde_json) or the checker enables a feature of a shared crate, metadata adds packages that the scan never covers. Reproduced: serde_json, itoa, memchr and zmij appear. The spec does not say what a closure package missing from the scan produces. Fix: take the closure from `cargo tree -p quire-eval -e normal,no-proc-macro --target thumbv7em-none-eabi` (per-package features, proc-macros excluded), and make a closure package absent from the scan an `unscanned` finding. | NFR-016.md:40-53,128-131,148-150 |
| FND-004 | medium | [major] `unscanned` covers only packages without metrics. geiger also reports `used_but_not_scanned_files`: build-script output compiled into the crate (serde, serde_core and thiserror `out/private.rs` in the run above). That code is linked and never scanned. Owner decision: fail on it, which fails today on serde and thiserror, or name it as outside the claim. | NFR-016.md:148-150 |
| FND-005 | medium | [minor] AC-3's scratch-copy check adds `unsafe {}` to a quire-eval source file. quire-eval forbids `unsafe_code` through `[workspace.lints.rust]`, so the build geiger runs fails to compile. The gate then fails through `unscanned` (scan exited non-zero), not through `first-party-unsafe`, and the check would pass even if detection were broken. Put the unsafe code where the build does not compile it (a `.rs` file outside the module tree), or assert on the `first-party-unsafe quire-eval` finding line. | NFR-016.md:194; TC-1131.md:45-46,61 |
| FND-006 | medium | [minor] The fixtures cannot express the forbid declaration. TC-1131 defines each fixture as a geiger report, a `cargo metadata` closure and an inventory. Neither JSON carries lint tables, and geiger's `forbids_unsafe` is false for crates that forbid only in the manifest (quire-semantic-value, quire-walk, quire-checked in the run above). The checker has to read manifests, workspace manifests and crate roots. AC-2 tests only the "declares neither" case. No fixture covers `[lints] workspace = true` inherited from a git dependency's workspace root (quire-canonical: workspace=true with no rust lints plus `#![forbid(unsafe_code)]`), `unsafe_code = "deny"`, or a workspace-member path crate. Add manifest and crate-root files to the fixture definition, plus those positive and negative cases. | NFR-016.md:193; TC-1131.md:17-27 |
| FND-007 | low | [minor] No finding kind covers an inventory entry that names a first-party crate. "The inventory admits no first-party crate", but stale-entry, reached-through-mismatch and malformed-entry do not cover it. TC-1131 step 3 expects only `first-party-unsafe quire-checked` with such an entry present, which rests on unspecified behaviour. Make an entry for a first-party crate a `malformed-entry`. | NFR-016.md:91-93,141-151; TC-1131.md:33-34,52 |
| FND-008 | low | [minor] `reached_through` is graph data derived from Cargo.lock and copied into the inventory, with a finding kind whose only job is to keep that copy in sync. A dependency refactor between first-party crates then forces inventory edits with no change in what unsafe code is linked. The checker already prints the set on `unlisted-unsafe`. Owner decision against the CLAUDE.md rule that versions and records live in lockfiles only: drop the key and `reached-through-mismatch`, or keep it as a review aid. | NFR-016.md:110,140-144 |
| FND-009 | low | [minor] "`quire-eval` has no features (FR-401)" overstates FR-401. FR-401 Behavior 3 forbids a `std` feature, and FR-403-AC-1 forbids a feature that adds or removes the evaluator. Say "has no `std` feature" or cite both. | NFR-016.md:46-47 |
| FND-010 | low | [minor] The gate scans `quire-eval`, which does not exist yet in agent-ix/quire-eval main 57712bd93 (members: quire-checked only), so AC-1 depends on FR-403 (S6) landing. NFR-016 lists FR-403 as `references`. Make it `depends_on`. | NFR-016.md:14-15 |
| FND-011 | low | [minor] AD-001 calls S8 "the unsafe/dependency audit". NFR-016 specifies only the unsafe half and says nothing about the dependency half: the sources, licences and advisories that quire-eval's existing `make deny` gates. State that S8's dependency audit is that existing gate, or place it out of scope. | spec/assurance/AD-001-shared-evaluation-core-interface.md:201,222-230 |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-012 | low | [minor] The first-party shipped-source scan counts "every module file those roots declare", but does not say how the checker resolves a `#[path = "..."]` module attribute or a source-file `include!` (not under `OUT_DIR`). `#[path]` is used in the agent-ix ecosystem today (quire-contract-ir's corpus generator binary). A checker that follows only default `mod` file resolution would miss a `#[path]` file or an included file, and could hold unsafe code without a finding. Fix: say that a `#[path]` module's file and an `include!`d source file outside `OUT_DIR` are shipped source, or that either one is an `unscanned` finding; add one fixture. | spec/non-functional/NFR-016-audit-unsafe-code-in-the-evaluator-closure.md:85-88 |
| FND-013 | medium | [minor] TC-1131 fixes the closure input in `cargo tree --prefix none` form. That form prints a flat package list with no depth or parent, so it carries no edges. `unlisted-unsafe` must name "the first-party crates that reach it through third-party crates only, read from the closure" (NFR-016 Gate 3; AC-4 expects `quire-exact` to be named for `big`). That set cannot be computed from the specified input. Fix: specify the closure as `cargo tree --prefix depth` (depth-annotated lines give the parent edges), or another form that carries edges. Use it in TC-1131's fixture definition and the checker's input. | spec/test-cases/TC-1131-audit-unsafe-checker-refuses-each-finding-kind.md:20; spec/non-functional/NFR-016-audit-unsafe-code-in-the-evaluator-closure.md:189-192,258 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | First-party crates are judged by the checker's own parse of the shipped targets (library, binary and build-script roots plus declared modules, whatever the `cfg`); `tests/`, `benches/` and `examples/` are not counted. Third-party crates are counted per package by geiger. `quire-canonical`'s unsafe code is only in tests/memory.rs and tests/drop_value.rs; its src/ holds only `#![forbid(unsafe_code)]`; so it passes. Lead default, flagged in the PR body. |
| FND-002 | fixed | The scan is specified by its inputs: the package's manifest by absolute path (the root is a virtual manifest), target, `--locked`, JSON. TC-1130 step 3 uses `--manifest-path <absolute path>`. |
| FND-003 | fixed | The closure is `cargo tree --locked -p quire-eval -e normal,no-proc-macro --target thumbv7em-none-eabi`; `cargo metadata` only locates files; a closure package absent from the scan's `packages` is `unscanned`. |
| FND-004 | fixed | Build-script `OUT_DIR` code is placed outside the claim, and AC-7 and TC-1131 step 8 pin "no finding". Lead default, flagged in the PR body. |
| FND-005 | fixed | The probe is `#[cfg(any())] mod probe;` with `unsafe {}` in probe.rs. The build compiles nothing new, so the gate fails through `first-party-unsafe quire-eval` naming probe.rs. |
| FND-006 | fixed | Fixtures carry first-party source trees (manifest, workspace root, sources). TC-1131 step 2 covers the own forbid, the table form with `priority`, `workspace = true` inheritance from a git-checkout root, `workspace = true` plus a crate-root attribute, `deny` only, and nothing. |
| FND-007 | fixed | An entry naming a first-party crate is `malformed-entry` (Gate 5, AC-6, TC-1131 step 6). |
| FND-008 | fixed | `reached_through` and `reached-through-mismatch` are removed; entries are exactly name, purpose and acceptance, and a `reached_through` key is `malformed-entry`. Lead default, flagged in the PR body. |
| FND-009 | fixed | Now "no `std` feature (FR-401) and no feature that adds or removes the evaluator (FR-403)". |
| FND-010 | fixed | The FR-403 relationship is now `depends_on`, and Dependencies says the gate passes only once quire-eval exists. |
| FND-011 | fixed | Scope and AD-001 place S8's dependency half in quire-eval's existing `make deny` gate. |
