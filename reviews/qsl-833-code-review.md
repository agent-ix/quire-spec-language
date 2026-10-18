---
id: SR-12121
title: "Code review of quire-spec-language PR #952: delete the native lower command and the lowering module"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@283f23cc045b70b1d889972e0f372d333f45630a; PR #952 diff against merge-base c9f351ddb: src/{cli,command,lib,main}.rs; src/command/{compilation,output}.rs; src/command/output/types.rs; src/lowering.rs and src/lowering/ (removed); src/command/projection_error.rs (removed); qsl-foundation/src/diagnostic.rs; qsl-semantics/src/check/capability.rs; tests/it/{compile_command,main}.rs; tests/it/{lower_command,native_lowering,integer_lowering,config_version_refusals,state_scalar_lowering}.rs (removed); README.md; docs/{native-error-codes,native-standalone}.md; examples/config-version/README.md; spec/spec.md; spec FR-010, FR-019, FR-075, FR-273, IT-002, NFR-003, NFR-004, StR-001, US-004, US-011; deleted FR-009, FR-029, FR-033, FR-034, TC-092, TC-093, TC-107, TC-111, TC-112, TC-912, Plan-008, Plan-009 Task-028 and their review records"
review_set: subset
---
# Code review of quire-spec-language PR #952

## Summary

Ticket: QSL-833. One code-review pass with the rust-review lane folded in,
over `git diff origin/main...HEAD` (102 files, +82/-6736). Read-only: nothing
was built or run, at the dispatcher's instruction. Every finding below was
found by reading the code at the reviewed sha.

Checked and clean:

- Authority: ADR-011 section 6.2 (`lowering` row) states that `lowering`,
  `lowering::target`, `ProjectionTarget` and `--target` are deleted with the
  `lower` command in M-6c (owner ruling 2026-09-24), and ADR-012 states the
  backend CLI argument is resolved to `BackendId` only. The deleted FR-009,
  FR-029, FR-033, FR-034 and their TCs specify exactly that retired
  capability, so the deletion follows the target design and is not a
  requirement dropped because code lags.
- No remaining importer of `lowering::*`, `ProjectionTarget`,
  `UnknownProjectionTarget`, `LoweringLimits`, `LoweringError`,
  `NativeProjection` (the root-crate one), `command::{lower, lower_for,
  compile_with, ProjectionLocation, ProjectionSource}`, `RunCause::Lowering`,
  `Stage::Lower`, `Details::Lowering`, `SpanStatus` or `compilation::package`
  in `src`, `qsl-*`, `tests`, `xtask`, `tools`, `Makefile`, `Cargo.toml`.
  `qsl-semantics::check::lowering` and `src/package/*::NativeProjection` are
  unrelated items with the same names.
- No Cargo feature, xtask import-graph entry, arch-lint exemption or shim
  names the deleted module. No compatibility layer, alias or fallback was
  left behind: `lower` is simply an unknown command.
- `main.rs`: the usage-error mapping now maps every usage error to refusal
  (exit 20); the only unsupported (21) usage error was `UnknownTarget`, which
  is gone. FR-010 AC-3 and AC-9 were narrowed to match, and AC-9's remaining
  clause is still tested (`tests/it/cli.rs:229`, `:266`).
- `projection_binding`'s only producer was `lowering`; removing it from
  `Code`, `as_str`, `all()` and the docs table is consistent.
  `invalid_projection_correspondence` still has producers
  (`src/complete/editor.rs`), so keeping it is right.
- `command.rs`, `compilation.rs`, `output.rs`, `output/types.rs`: removed
  imports match removed uses (`NativePackage`, `Span`); `compile` still uses
  `source_only` and `package_of`.
- Spec edges: no remaining `relationships` entry, in spec or review records,
  targets a deleted `quire-spec-language` id. FR-075 and US-011 were
  rewritten in present tense.
- Deleted tests trace only deleted FR/TC ids, except one
  `integer_lowering` case tagged FR-010-AC-9 for the unknown-target exit,
  whose AC clause this PR removes.

## Verdict

FIX. The deletion itself is correct, justified by the owner ruling, and
leaves no shim. Three blocking defects stop the workspace from building or
its tests from passing: the catalog array length (FND-001), a duplicated
test function (FND-002), and a surviving test that still runs `lower`
(FND-003). FND-004 to FND-006 are low and recorded, not blocking.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | `CATALOG_CATEGORIES` is still declared `[(&str, Category); 50]` but now holds 49 entries after `("projection_binding", Category::Refusal)` was removed. Rust requires an exact array length, so `qsl-foundation` fails to compile (E0308, expected an array of 50 elements, found 49), and every crate in the workspace with it. Fix: declare the length as 49. | qsl-foundation/src/diagnostic.rs:980 |
| FND-002 | high | `tests/it/compile_command.rs` defines `fn a_complete_v1_program_compiles_through_the_spine` twice, at lines 342 and 397 with identical bodies. Commit 31704576e inserted a second copy after the new `lower_is_an_unknown_command_...` test. Two items with one name in a module is E0428, so the `it` integration test crate does not compile. Fix: delete the second copy (lines 392-420). | tests/it/compile_command.rs:342; tests/it/compile_command.rs:397 |
| FND-003 | high | `lower_refuses_a_complete_v1_program_as_unknown_edition` still runs `quire-spec lower <compile.json>` and parses stderr as JSON. `lower` is now an unknown command, so stderr is the plain line `unknown command: lower`, and `serde_json::from_slice(&output.stderr).unwrap()` panics. The test is tagged `TC-107`, `FR-029-AC-3`, both deleted by this PR. Fix: delete the test (lines 422-460). | tests/it/compile_command.rs:422-460 |
| FND-004 | low | FR-031's Behavior still says the command refuses extraction combined with "source-only compile/lower export". There is no `lower` export now; the sentence should name `compile` only, matching the updated `extraction-requires-run` row in docs/native-error-codes.md. | spec/functional/FR-031-run-extracted-native-source.md:63 |
| FND-005 | low | Stale prose that still names the deleted capability: README.md:248 says `MappedPackage` exposes the package for "validation, evaluation and lowering"; xtask/src/import_graph.rs:386 lists `lowering` among later-layer modules in crates depending on `qsl-semantics`; docs/native-linked-packages.md:183,191 describe LC04 lowering, which spec.md section 2.1 no longer lists in scope; ADR-030 lines 435-436 list FR-009, FR-033 and FR-034 among the native-profile requirements. | README.md:248; xtask/src/import_graph.rs:386; docs/native-linked-packages.md:183; spec/decisions/ADR-030-arbitrary-nesting-depth-no-fixed-caps.md:435 |
| FND-006 | low | The new test's second half asserts the repository layout (`src/lowering.rs` and `src/lowering/` do not exist, `lib.rs` does not contain `mod lowering`). That checks files, not behaviour, and the compiler already guarantees no caller reaches a deleted module. The CLI half (`lower` exits 20 with `unknown command: lower`) is the behaviour FR-010-AC-3 states; the file-existence asserts can go. | tests/it/compile_command.rs:385-389 |
