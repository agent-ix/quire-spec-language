---
id: SR-20261107003000
title: QSL-15 ARCH-G4 gate pass and QSL-462 ARCH-G4E evaluation
type: SpecReview
analysis: architecture-evaluation
scope: "agent-ix/quire-spec-language origin/main f11622cf3 (after #2551); QSL-15 and QSL-462; docs/deletion-gate-evaluation-checklist.md (both sections); ADR-011 SEAM-1 rule and deletion order; quire-protocol origin/main 2cdd65a, built at its own lock and against QSL f11622cf3"
review_set: subset
---

## Summary

This review measures every QSL-15 and QSL-462 checklist row again on origin/main f11622cf3, in a
read-only detached worktree. The previous run (SR-20261107002000, #2550) failed only on the
lanes rule: the root `0-draft` front end was still public. QSL-1810 (#2551) deleted it.

Verdict for QSL-15: **PASS**. Verdict for QSL-462: **PASS**. Its precondition, QSL-15 passed, now
holds.

QSL-15 rows:

- Row 1 PASS. `git ls-files` is empty for `src/native_model`, `src/protocol_artifact`,
  `src/temporal`, the composed paths, `src/parser*`, `src/syntax*`, `src/linking*`,
  `src/checking*` and the four deleted docs. The row 1 `git grep` finds only by-absence tests,
  qsl-eval's own `temporal` successor, and qsl-package's `protocol_artifact_v1`/`_v2` vocabulary
  tables (FND-004).
- Lanes rule PASS. The root crate now holds `command`, `complete`, `emit_checked_v2`,
  `formal_source`, `checked_handoff`, `population_fixtures` and, until this PR, `number`. None is
  SEAM-1. `command` is the binary's spine compiler and runner. `formal_source` is used by
  `command`. `checked_handoff` and `emit_checked_v2` are used by quire-protocol. `complete` is
  editor tooling over `qsl-cst`. `population_fixtures` backs an example. The one module left
  without consumers, `number`, is a FR-511 re-export, not SEAM-1, and is deleted here (FND-001).
- Row 3 PASS. The 35 criterion ids whose tags #2551 removed are all retired, absent from the
  spec, or method-without-symbol. Every criterion on QSL-1810's list is tagged or retired, and
  FR-013-AC-3 is tagged. `quire matrix --scope .` reports 3366 tagged, 1042 untagged, 96
  method-without-symbol and 3 tagged-by-ignored-test.
- Row 4 PASS. `arch-lint qualified-core` passes both direction and ambient input.
  `tc_244_only_the_named_stage_constructors_return_stage_types` passes, as do the other
  typestate scans.
- Row 5 PASS. 388 tests tagged to the nine G4 scenario families pass by name: S1 FR-063/074,
  S2 FR-316/318/322, S3 FR-083/084/120/300, S4 FR-327/329/330, S5 FR-114/115/116, S6
  FR-335/057, S7 FR-340/343/345, S8 FR-304/306, S9 FR-263/183/121.
- Rows 6 and 9 PASS. 97 tests tagged to FR-327-AC-1x, FR-329, FR-501 and FR-502 pass, and so
  do 12 tests tagged to FR-526-AC-2x.
- Row 7 PASS. The glob grep finds only `qsl-eval/src/value/expression/mod.rs:2346`, TC-385's
  test-only enum-to-name function, which is not dispatch. The positive control matches under
  both `qsl-eval/src/` and `src/`.
- Row 8 PASS. quire-protocol main names only `checked_handoff` and `emit_checked_v2` from the
  root crate. Its `cargo check --locked --all-targets` passes, and so do its lib, TC-003, TC-011
  and TC-149 to TC-153 tests (78 tests) at its lock (QSL 27adf2f). FND-003 records what happens
  when it moves to current main.

Workspace run: `cargo test --workspace --all-features --no-fail-fast`. The only failures are the
accepted baselines: 29 in the qsl-package lib (QSpec #633 drift; the lib was re-run alone at
`--test-threads=2` after it was SIGKILLed for memory in the full run) and 6 in xtask
`qualification_adapters`. Nothing else is red.

QSL-462 rows, all PASS:

1. EN-1: the `lasso_replay` suite passes, including
   `a_liveness_violation_settles_refuted_through_replay`.
2. Tampered certificates are rejected: 59 tamper and reject tests pass across the closure,
   component, zone, zone-live, refinement, timed-refinement, probability and state-graph
   checkers.
3. Uncertified engines are labelled: 7 tests pass, including
   `a_sound_unsat_settles_proved_uncertified_with_horizon_and_jumps`.
4. Undefined and budget outcomes: 87 tests tagged to FR-283, FR-330, FR-523 and FR-203 pass,
   with `an_undefined_sample_settles_measured_rejected`,
   `an_undefined_tuple_refutes_and_an_undetermined_match_is_inconclusive` and
   `a_horizon_settles_inconclusive_and_the_record_states_what_it_covers`.
5. `arch-lint` shows no core to `qsl-analyze` edge.
6. `reduced_and_unreduced_runs_agree_over_the_corpus` passes.
7. All 70 sub-issues are Done.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Compatibility re-export past its own end condition. `src/number.rs` re-exported `qsl_package::number` only "while the external quire-protocol consumer imports `quire_spec_language::number::*`" (FR-511). That end condition holds: quire-protocol main names the path only in a by-absence test (`tests/tc_153_end_retired_surfaces.rs:47`), and GitHub code search finds no other user. It was the one exemption to FR-087-AC-9's no-re-export rule, and the only root module with no consumer but its own test. Fixed here: the module, its root test and the TC-256 scanner exemption are deleted, and FR-511-AC-1 is retired. FR-511, FR-087-AC-9, TC-256, TC-1306, ADR-011 §1.9 and §7.2, FR-499 and FR-521 are restated. FR-038 stays live, because FR-100, FR-106 and FR-286 cite its integer spelling. | src/number.rs; spec/functional/FR-511-keep-the-exact-protocol-number-codec-as-a-root-module.md; tests/it/layer_crate_reexports.rs:229-306,407-512 |
| FND-002 | low | Stale prose. The root crate doc still read "Native syntax, formatting and source diagnostics for the finite-state profile" after #2551 deleted the front end. Checklist row 8 cited IT-001 and IT-002, which have no test in quire-protocol (they are its FR-047-AC-3 work). Fixed: the crate doc is rewritten, and row 8 names the tests that exist. | src/lib.rs:2-4; docs/deletion-gate-evaluation-checklist.md row 8 |
| FND-003 | low | Risk, not a QSL defect: quire-protocol's next lock bump will break. Pointed at QSL f11622cf3, `tests/tc_152_temporal_evidence.rs:71` does not compile (`EvidenceRequest` has no field `native_valuations`). With quire-contract-ir and quire-canonical moved to QSL's own pins, `tc_149` panics because the IR operation catalog cannot read `carried_member`. That second failure comes from a hand-mixed lock. A full `cargo update` also fails, because QSL main's `qsl-semantics` does not cover IR main's new `CheckedNodePathSegment::Operand`. Row 8 is met at quire-protocol's own lock. The consumer fix belongs to quire-protocol's lock bump, as QSL-1800 did last time, and the IR variant belongs to QSL's next IR pin bump. | quire-protocol tests/tc_152_temporal_evidence.rs:71; quire-contract-model checked_package/v2/operation_catalog.rs:278 |
| FND-004 | low | Retained vocabulary with no reader or consumer. `qsl_package::number` (the FR-038 codec), `qsl_package::protocol_artifact_v1` (FR-513-AC-16) and `protocol_artifact_v2::RefusalCode` (FR-513-AC-17) serve no QSL producer or reader and no agent-ix consumer. Only their own tests use them. QSL-1808 kept them by a ruling made the day before this review, and FR-038's integer spelling is cited normatively, so this review does not delete them. Retiring them is a separate spec decision, not a QSL-15 path. | qsl-package/src/lib.rs:43-48; spec/functional/FR-513-name-the-successor-or-keep-of-each-retained-criterion-that-rides-a-retired-path.md:81-82 |

## Evidence and Limits

The eval worktree is detached at f11622cf3. QSpec was read from qspec-main-terra at 7e63321f. The
tools were quire 0.36.2 (engine 0.50.2). Every cargo run went one at a time through
locked-build.sh, with a private target dir. During the run, origin/main moved to 786eb8f14
(#2552, an FR-230 test), which touches none of the rows. This time quire-protocol was built and
tested, at its own lock and against QSL f11622cf3; nothing was committed there. The FND-001 fix
was checked with the `layer_crate_reexports` and `spine_deletion_boundaries` tests and with
clippy `-D warnings` on the root crate, all features and all targets. No numeric score is given.
