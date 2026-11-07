---
id: SR-20261107001500
title: QSL-15 ARCH-G4 semantic-facet and deletion gate evaluation
type: SpecReview
analysis: architecture-evaluation
scope: "agent-ix/quire-spec-language origin/main 122aafd1f; QSL-15; docs/deletion-gate-evaluation-checklist.md (QSL-15 section); ADR-011 deletion order steps 0 to 7; quire-protocol origin/main 2cdd65a (Cargo.toml and use sites only)"
review_set: subset
---

## Summary

This review measures every QSL-15 pass condition and checklist assertion on origin/main
122aafd1f, using a read-only detached worktree. It runs the checklist greps, `arch-lint
qualified-core`, the named successor tests (by name), `quire matrix --scope .` and the
Linear states of every gate ticket. Verdict: **FAIL**. Two old-architecture gaps block the
pass: the `protocol_artifact` B9 remainder still ships, and 11 retained criteria lost their
only tests in the deletion train. Neither gap is a Layer 1/2/3 interface decision. Both now
have owned corrective tickets under Layer 4: QSL-1808 and QSL-1809, each blocking QSL-15.

What passes:

- `src/native_model`, `src/temporal` and the composed paths are absent.
- Every step-0 gate ticket is Done: QSL-1549, 1495, 1599 (In Progress, consumer-only),
  1446, 1595, 1596, 1582 and 1380. QSL-1599 is not a QSL path. Every QSL-15 blocked-by
  edge is Done or Canceled.
- quire-protocol main reads the checked package through the IR reader. It has no
  `handoff-writer` feature and no `protocol_artifact` use.
- `arch-lint qualified-core` passes both direction and ambient input.
- The row 7 string-dispatch grep finds nothing, and its positive control (a scratch
  `"StateModel"` file under `src/`) matches.
- These successor tests pass by name: FR-327-AC-6..13, FR-329-AC-4/8/9/10, FR-507-AC-6/7/8/11,
  FR-509-AC-6/7/8, FR-517-AC-11..13, FR-526-AC-20..28, FR-501, FR-502, TC-1293 and
  FR-499-AC-5. The earlier #2315 known inputs are all now tagged, and QSL-1639 is Done.
- Nine G4 scenarios each have implemented, tagged and passing spine evidence (171 sampled
  tests green). The scenarios are sum/case forms, model simulation, temporal trace and lasso,
  protocol anchors and frame replay, unbounded declarations, refinement versioning, the
  abstraction relation, call-site replay and family outcome layering.

Also found and fixed in this PR:

- `docs/deletion-table.md` was a 329-row file-tracking ledger, and checklist assertion 2 gated
  on it. This PR deletes both and repoints ADR-011 and the checklist to `quire matrix`.
- TC-244 (FR-087-AC-1/2) was red on main: the expected set still held the deleted
  `src/checking.rs::check`.
- FR-499-AC-4 had no tag.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Checklist assertion 1 and pass condition "no second package path" FAIL: `src/protocol_artifact` (26 files, `pub mod` in `src/lib.rs`) remains, with the /1 to /3 readers and `encode_candidate`. QSL-838 owns these files and is Done. Only QSL's own tests consume them. FR-513-AC-15 (#2522) keeps 9 criteria (FR-050-AC-2/3, FR-054-AC-1..4, FR-049-AC-2/5, FR-513-AC-17) whose only tests are legacy-reader tests. `RegisteredDefinition::Protocol` is still public, which contradicts FR-499-AC-2, and `protocol_artifact::wire` imports remain, which contradicts FR-499-AC-3. Repair: port or retire the 9 criteria, then delete the module, its tests and `docs/compiled-protocol-v1.md`/`-v2.md`. Owned by QSL-1808. | src/lib.rs:17; src/protocol_artifact/mod.rs; tests/it/retained_criteria_matrix.rs; spec/functional/FR-499-delete-each-retired-family-only-behind-its-named-successor.md:63-66,121-136; FR-521 rows 7-9,12,15 |
| FND-002 | high | Checklist assertion 3 FAIL (FR-499-AC-6). The deletion PRs #2518 and #2523 deleted the only tagged tests of 11 retained criteria, and `quire matrix` lists them untagged: FR-035-AC-2/3/6, FR-040-AC-4/8, FR-047-AC-6, FR-048-AC-2/3, FR-056-AC-8, FR-057-AC-11 and FR-513-AC-8. `architecture_fixture_binds_references_through_spine_i2` was already a spine test and was deleted only because of its file name (`composed_domain_models.rs`). Measured as: untagged now, and trace-tagged in a since-deleted file at `e88993ffe^`. Owned by QSL-1809. | spec/functional/FR-035-parse-composed-native-units.md:70-74; FR-513 row FR-056-AC-8; git show e88993ffe^:tests/it/composed_domain_models.rs:519 |
| FND-003 | medium | File-tracking antipattern: `docs/deletion-table.md` was a hand-maintained 329-row status ledger, and checklist assertion 2 gated QSL-15 on it. A ledger can read green while the matrix is red, as FND-002 shows, so it guards nothing. Fixed in this PR: the table and assertion 2 are deleted, and ADR-011, assertion 3/5 evidence and spec reference bullets now cite `quire matrix`. | docs/deletion-gate-evaluation-checklist.md; spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md:1297 |
| FND-004 | medium | TC-244 `tc_244_only_the_named_stage_constructors_return_stage_types` (FR-087-AC-1/2) fails on main. It expects `src/checking.rs::check`, which QSL-499 deleted. This leaves the assertion 4 evidence red. Fixed in this PR by dropping the stale expectation. | xtask/src/typestate_scan.rs:851 |
| FND-005 | low | FR-499-AC-4 (native_model absent and TC-469 passes) had no tag. Fixed: a new by-absence test, `the_native_model_module_is_absent`, is tagged FR-499-AC-4 and TC-1293 and was checked with a positive control. The TC-469 walkthrough test is also tagged FR-499-AC-4. `docs/native-model-checking.md`, which documents the deleted API, is deleted. | tests/it/spine_deletion_boundaries.rs; tests/it/config_version_spine.rs:325 |
| FND-006 | low | Risk, not a checklist row: the root crate still exports the historical `0-draft` parser, `syntax` (`ix:native`), `linking::link` over `quire_contract_model` and `checking::NativeType`. Only tests and `protocol_artifact` consume them. These are SEAM-1 remnants outside checklist assertion 1's path list. Whether they go with QSL-1808 needs an owner call. Stale `native_model` prose remains in qsl-foundation/src/diagnostic.rs:498, qsl-package/src/lib.rs:11 and qsl-semantics/src/value/mod.rs:5. | src/lib.rs; src/parser.rs:10; src/linking.rs |

## Evidence and Limits

The eval worktree is detached at 122aafd1f. origin/main moved to 97a2a5abf during the run.
The three new commits touch none of the findings, except #2522, which adds FR-513-AC-15
(the cause of FND-001's keep). QSpec was read from qspec-main-terra at f96cb5c1. Tools:
quire 0.36.2 (engine 0.50.2), and `quire matrix --scope .` returned 3353 tagged, 1041
untagged, 96 method-without-symbol and 3 ignored. Each cargo run went through
locked-build.sh with a shared target. The qsl-package conformance and qualification_adapters
failures known on main were not chased, per the owner. quire-protocol was inspected, not
built: FR-521-AC-3 says QSL runs no build of that repository. QSL-1740 (FR-044 linker, In
Progress) is consumer-side work. Nothing here is a numeric architecture score.
