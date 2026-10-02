---
id: SR-1233
title: "Code review of quire-spec-language PR #609: qualified-core in make ci, OnceLock caches removed (LC6b)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@3f607547271412ef60ff235ea429d454edd028cd; PR #609 diff against origin/main: Makefile, qsl-package/src/emit.rs, qsl-semantics/src/value/{definition,diagnostics_catalog,mod}.rs, tools/arch-lint/{qualified_core,api_surface}.rs, src/linking/composed/definition_source.rs, tests/it/{compiled_protocol_v2,composed_definition_source}.rs, FR-284, spec.md, tests.md"
review_set: subset
---
# Code review of quire-spec-language PR #609

## Summary

Ticket: QSL-594 (LC6b). The PR does four things:
- It removes both `OnceLock` caches.
- It keeps the FCD lift scratch directory.
- It adds `arch-lint-qualified-core` to `ci:`, with two exact exemptions.
- It replaces FR-284's known-violation list.

What the coordinator asked to check:
- **OnceLock removal.** `DefinitionLock::pinned()` and `native_diagnostics_catalog()` now return owned values. The new `native_diagnostics_identity()` borrows from the `'static` bytes.
  - The diff adds no `static`, `thread_local!`, once-cell or lazy value. The live `arch-lint qualified-core` run, which now flags once-cells, passes.
  - Per-call cost: the production callers are `resolve_profiles` (once per compile), `emit_package_inner` (once per emit, reading the lock once and passing it down) and `diagnostics_catalog()` (once per emit).
    - Each call parses the 6.8 KB lock JSON, or hashes the 33 KB `native-diagnostics.md`.
    - No call sits in a per-node or per-state loop. `qsl-bench` `text_profile()` runs once per generated package.
- **The FCD evidence holds.** At rev `033e228` (the one QSL locks) and on FCD main `4003d4dc`:
  - `LiftRequest.out: PathBuf` is required.
  - `lift()` returns `LiftOutcome::Written{document}` only after `write_lift` succeeds.
  - `clap` is an unconditional dependency, with no `[features]`.
  - So the scratch directory exists only to feed `lift`, which is the plan lead's condition for keeping it.
- **The two consts.** These are judged under FND-001 and FND-002.
  - `FCD_LIFT_CLAP` is skipped only for the edge `agent-ix-extraction-frontend -> clap` whose frontend was reached directly from `qsl-semantics`, and `clap` is not marked seen.
  - `FCD_LIFT_SCRATCH` matches file, enclosing function and category exactly.
  - The tests show that the other paths fail: a direct `clap`, `clap` through another crate, another core crate reaching the frontend directly, another category inside `lift_document`, filesystem access in another function of the file, and a same-named function in another file.
- **The `references` drop from 2581 to 2580 is a sort-position effect, not a behaviour change.**
  - The handoff dependencies are sorted by key, and `metadata::dependency` is a linear search that charges one `visit` per entry until it finds a match.
  - The old rule paths both began `https://…`. The new ones are `docs/…`, which sorts before that, and `qsl-foundation/…`, which sorts after the keys that are looked up. One lookup therefore passes one fewer entry.
  - Verified locally: renaming the path to `aqsl-foundation/src/diagnostic.rs`, so that it sorts first again, restores `references` to 2581. That edit was reverted, and the tree is clean.
- **Repo-relative `rule!` paths (team-leader ruling).** Both new paths exist in this repository.
- **Collisions.**
  - #598 (head 74769baa) edits the same `ci:` line, adding `checked-input`. That is a textual conflict: whichever PR merges second keeps both targets.
  - #598 touches only xtask and spec files, so it cannot trip the new gate.
- **Gates run on the head.**
  - All 102 arch-lint tests pass.
  - The live `arch-lint qualified-core` run passes.
  - The qsl-semantics and qsl-package tests pass.
  - The `composed_definition_source` and `compiled_protocol_v2` tests pass (19).
  - Clippy `-D warnings` is clean for qsl-semantics, qsl-package, arch-lint and the root crate.

## Verdict

Changes requested: one medium finding and one low.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `FCD_LIFT_CLAP` and `FCD_LIFT_SCRATCH` are a two-entry allow-list. They are as narrow as an exemption can be, and both causes are FCD's (verified above), but the plan lead's rule is "no allow-list", so keeping them needs the plan lead's explicit acceptance as a temporary exception. Separately, the PR deletes their exit conditions. The old Status said "Fix: put `clap` behind a feature in filament-core-data" and "Fix: a `lift` in filament-core-data that returns the document bytes". Neither the new Status nor the const docs state when the exemptions end, so they now read as permanent. Fix: restore the two exit conditions in FR-284's Status and in each const's doc comment; open the FCD Linear ticket that carries them; and delete both consts, with their tests, once FCD ships. | tools/arch-lint/qualified_core.rs:110-124; spec/functional/FR-284-keep-the-qualified-core-separable-by-crate.md:93-104 |
| FND-002 | low | The `clap` skip depends on breadth-first discovery order. `is_fcd_lift_clap` reads the single `previous[frontend]`. If a core crate reaches `agent-ix-extraction-frontend` both directly from `qsl-semantics` and through another crate (for example `qsl-semantics -> Y -> frontend`), the first discovery wins. Every later route is then marked seen, so that route's `clap` is skipped too. FR-284's Status and the module doc say "Any other path from a core crate to `clap` fails", which overstates the check. Fix: refuse any edge into `agent-ix-extraction-frontend` from a crate other than `qsl-semantics`, so that `clap` under the frontend is always the one sanctioned edge. Alternatively, narrow the claim to "any path to `clap` not under `agent-ix-extraction-frontend`". | tools/arch-lint/qualified_core.rs:345-357; tools/arch-lint/qualified_core.rs:27-30; spec/functional/FR-284-keep-the-qualified-core-separable-by-crate.md:96-99 |
