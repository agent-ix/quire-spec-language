---
id: SR-20261107002000
title: QSL-15 ARCH-G4 gate re-evaluation and QSL-462 ARCH-G4E measurement
type: SpecReview
analysis: architecture-evaluation
scope: "agent-ix/quire-spec-language origin/main 7444b532b; QSL-15 and QSL-462; docs/deletion-gate-evaluation-checklist.md (QSL-15 and QSL-462 sections); ADR-011 deletion order; quire-protocol origin/main 2cdd65a (Cargo.toml and use sites only)"
review_set: subset
---

## Summary

This review measures every QSL-15 pass condition and checklist assertion again on origin/main
7444b532b, in a read-only detached worktree. The earlier evaluation (SR-20261107001500, #2533)
failed on two blockers. Both have landed: QSL-1808 (#2536, `src/protocol_artifact` deleted) and
QSL-1809 (#2537, 11 criteria re-owned). The run covered the checklist greps, `arch-lint
qualified-core`, `quire matrix --scope .` and one `cargo test --workspace --all-features
--no-fail-fast`. That run passed 8992 tests and failed 38: the 29 known qsl-package QSpec #633 drift
failures, the 6 known xtask `qualification_adapters` failures, and 3 new qsl-semantics failures
(FND-002). Doctests were re-run separately and pass.

Verdict for QSL-15: **FAIL**. One path blocker remains. The SEAM-1 `0-draft` front end (`parser`,
`syntax`, `linking`, `checking`) still ships from the root crate, and only its own tests use it
(FND-001, QSL-1810). The QSL-15 lanes rule is "a SEAM-1 module is deleted in the lane PR that
removes its last consumer", and ADR-033 kept this parser "with native `compile` until M-6c".
Native `compile` is gone.

What passes:

- Row 1: `git ls-files` is empty for `src/native_model`, `src/protocol_artifact`, `src/temporal`,
  the composed paths and the four deleted docs. The checklist `git grep` finds only by-absence
  tests, qsl-eval's own `temporal` module (the successor) and stale prose (fixed here). FR-499-AC-2
  to AC-5 (TC-1293) and FR-517-AC-10 (`the_legacy_writer_is_absent`) pass.
- Row 3: all 11 QSL-1809 criteria are tagged, except FR-035-AC-6, which is retired. One more
  criterion lost its only test in the train: FR-041-AC-5 (FND-003, fixed here).
- Row 4: `arch-lint qualified-core` passes direction and ambient input, and TC-244 passes.
- Row 5: the nine G4 scenarios' tagged spine tests pass. Outside the known qsl-package and xtask
  sets, the only workspace failures are the FND-002 intake tests.
- Rows 6 and 9: the FR-327, FR-329, FR-501, FR-502 and FR-526-AC-20..28 tests and lasso replay pass.
- Row 7: no string-matched family dispatch exists. The checklist grep was vacuous over the
  `qsl-*` crates, though (FND-004, fixed here).
- Row 8: quire-protocol main uses only `checked_handoff::Selection`, `Corpus`, `emit_checked_v2`
  and the qsl-* crates. It has no `handoff-writer` and no `protocol_artifact` or `native_model` use.

QSL-462 was measured, not passed, because its precondition (QSL-15 passed) does not hold. Every
row measures green:

- EN-1 replayed counterexamples: `lasso_replay` and `a_liveness_violation_settles_refuted_through_replay`.
- Tampered certificates are rejected by the closure, component, zone, zone-live, refinement,
  timed-refinement, probability and SCC checkers.
- `uncertified` labels are carried, including on hybrid.
- An undefined sample settles `measured: rejected`, and an undefined hyper tuple refutes.
- A budget or horizon settles inconclusive, and the FR-203 defaults are published.
- arch-lint shows no core to `qsl-analyze` edge.
- `reduced_and_unreduced_runs_agree_over_the_corpus` passes.
- Every sub-issue is Done, including QSL-577 and QSL-578.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | QSL-15 FAIL (lanes rule, row 4 "no second path"). The historical `0-draft` front end is still public in the root crate: `parser` (`parse`, `parse_source`: "Parse the historical `0-draft` grammar"), arena `syntax`, native `linking` (`link`, `LinkLimits`, `LinkedPackage` over `quire_contract_model`) and `checking::NativeType`. About 2,480 lines in all. No binary entry point, spine crate or agent-ix repository uses them (GitHub code search). Their only consumers are seven root test files, which carry about 45 criteria (FR-001..005, FR-013, FR-017, FR-035-AC-7, FR-088-AC-1, FR-096-AC-12, FR-134, FR-255, FR-256, FR-277). QSL-837 ("then delete the 0-draft parser") is Done without the deletion. Repair: delete the modules and re-own or retire their criteria. Owned by QSL-1810, which blocks QSL-15. | src/lib.rs:7,12,14,17,26-28; src/parser.rs:10; spec/decisions/ADR-033-one-source-text-for-the-checked-graph-and-the-type-report.md:141; spec/functional/FR-035-parse-composed-native-units.md:50; tests/it/parser.rs; tests/it/linking.rs |
| FND-002 | high | Main is red outside the known sets. #2544 moved `quire-canonical` to 288c94c01, which raises a repeated member name as the new `ReadError::DuplicateName { offset, pointer }` instead of `Malformed`. `PackageDocument::parse` fell to its catch-all ("could not be read"), and observation check 1.3 fell to `_ => None`, so a document with a repeated name was admitted (`Ok(())`). Three tests fail: FR-056-AC-12, FR-106-AC-3, FR-056-AC-2. Fixed in this PR by mapping `DuplicateName` to the malformed and invalid-value refusals. The 138 intake and observation tests now pass. | qsl-semantics/src/model/intake.rs:542; qsl-semantics/src/model/observation.rs:1604; Cargo.lock (quire-canonical 288c94c01) |
| FND-003 | medium | Row 3 (FR-499-AC-6): FR-041-AC-5 describes the deleted composed `bind_models` and lost its only tagged test in #2518 and #2523. FR-057-AC-14 (composed admission stage projection, TC-973) describes the deleted composed linker. Fixed: both are retired with their spine successors cited (FR-036-AC-9; FR-057-AC-5 and AC-11), and FR-041's composed-binder SHALL is restated as history. Measured by intersecting criterion ids removed from `#[trace]` lines in `5758035cc^..HEAD` with live untagged criteria. | spec/functional/FR-041-admit-rational-native-model-profile.md:99,127; spec/functional/FR-057-admit-shared-capability-kinds.md:394 |
| FND-004 | medium | The checklist row 7 grep pathspec `'qsl-*/src'` matches no file under `qsl-*/src/`, so it never scanned the spine crates. Its positive control sat under root `src/`, so the control passed while the grep was vacuous where it mattered. With `':(glob)qsl-*/src/**'` the grep finds one hit, `qsl-eval/src/value/expression/mod.rs:2345`. That hit is a test-only exhaustive enum-to-name function (TC-385), not dispatch. Fixed: the checklist now uses the glob pathspec, places the control in both trees, and states that rendering an enum variant as its name is not dispatch. | docs/deletion-gate-evaluation-checklist.md row 7 |
| FND-005 | low | `src/checking/proof/{facts,graph,walk}.rs` (720 lines) have been uncompiled orphans since #2523 removed `mod proof;`. Fixed: deleted. Stale prose about deleted modules is also fixed: `native_model` in qsl-foundation diagnostic.rs, qsl-package lib.rs and qsl-semantics value/mod.rs, and "the legacy protocol reader" in qsl-package protocol_artifact_v2.rs. The QSL-462 checklist row 7 and its "known inputs" paragraph were stale and are restated. TC-973 now covers only a retired criterion. | src/checking.rs; qsl-package/src/protocol_artifact_v2.rs:2 |

## Evidence and Limits

The eval worktree is detached at 7444b532b. QSpec was read from qspec-main-terra at 7e63321f. The
tools were quire 0.36.2 (engine 0.50.2). `quire matrix --scope .` reported 3377 tagged, 1031
untagged, 96 method-without-symbol and 3 tagged-by-ignored-test. Every cargo run went through
locked-build.sh with the shared target. In the workspace run, doctests failed with E0463 because
other worktrees were changing the shared target at the same time. A separate `--doc` run passed.
The known qsl-package and xtask failures were not chased, per the owner. quire-protocol was
inspected, not built: FR-521-AC-3 says QSL runs no build of that repository. No numeric score is
given.
