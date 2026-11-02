---
id: SR-2160001
title: "Code review and Rust review of the checked-package handoff writer (quire-spec-language PR 2160, QSL-1546)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; src/checked_handoff.rs, src/checked_handoff/{corpus,selection,tree}.rs, src/emit_checked_v2.rs, tests/it/checked_handoff.rs, examples/checked-handoff/**, Cargo.toml"
review_set: subset
evaluated_revision: "103461726b52387c4c4922c1b707b1fd3e400997"
---

# Code review and Rust review, quire-spec-language PR 2160

## Summary

Ticket: QSL-1546. Head 103461726b52387c4c4922c1b707b1fd3e400997 against base e9a388154, scope `git diff origin/main...HEAD`.
Examined: every FR-517-AC-1..10 and FR-518-AC-1..10 against code and tests; the `emit_checked_v2` refactor;
feature gating; panic and lint surface.

Gates run in the fresh clone (`CARGO_BUILD_JOBS=2 CARGO_INCREMENTAL=0`, through qsl-slot): `cargo check --workspace
--all-targets` pass; `cargo clippy --workspace --all-targets -- -D warnings` pass with default features and with
`--features checked-handoff`; `cargo check --workspace --no-default-features` pass; `cargo test --features
checked-handoff --test it checked_handoff` 18 passed, 0 failed, 2 ignored; `--test it emit_checked` 11 passed;
`make arch-lint-qualified-core arch-lint-api-surface-qsl` pass. No `#[allow]`, no `unwrap`/`expect` in production
paths of the new module, no digest, hash or pin copied outside Cargo (the corpus is a manifest; the example inputs
carry none), `Cargo.lock` unchanged.

Mutation evidence (reviewer-local, reverted): removing the `ClauseName`, `DefinitionIdentity` and `DefinitionRevision`
comparisons from `Selection::check` and the `environment.check` read-back call from `write_with` leaves all 18 tests
green (FND-001). Removing the temporary-directory cleanup in `tree::publish` turns the FR-517-AC-6 test and five
corpus tests red; disabling the `period` comparison turns `every_case_gets_the_refusal_of_its_row` red.
Expected values in the tests (`table()` in tests/it/checked_handoff.rs) are written by hand and are independent of
corpus.rs; `corpus::verify` checks each case against the real reader at write time, so the corpus is not circular.
The only non-independent case is 28m, where the test `Double` returns the same constants as the case (spec-sanctioned,
FR-518 "Moved cases").

Out of PR scope, listed in SR-2160002: spec and IR gaps.

## Verdict

NOT MERGEABLE as is: no high finding, three medium findings (FND-001 to FND-003) that need a fix or an explicit
ruling, plus low findings. FND-001 depends on the spec ruling in SR-2160003 FND-002.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | Selection axes `clause.name`, `definition.identity`, `definition.revision` and the write-time read-back `environment.check(&selection, &package)` are never exercised by any test: deleting all four leaves 18/18 tests green. Rows 8, 12 and 13 moved to the reader stage, so no corpus case fires those axes, and the only read-back failure test injects `read_back_differs` instead of making the comparison fail. | src/checked_handoff/selection.rs:check_clause,check_profile; src/checked_handoff.rs:write_with | correct-requirement-no-evidence |
| FND-002 | medium | The FR-517-AC-1 test asserts the directory holds `checked-package.json`, `models/`, `selection.json`, `sources/` only, but the Outputs table of FR-517 lists `inventory.json` (`qsl_replay::qsl_inventory`, FR-514) and AC-1 says "exactly the paths of the Outputs table". The test matches the code, not the spec; nothing is gated or ignored for the missing file (blocked on QSL-1512 / PR 2140, open). | tests/it/checked_handoff.rs:217; spec/functional/FR-517 Outputs | correct-requirement-no-evidence |
| FND-003 | medium | Recipe R is weakened in the port without a ticket: the `Full` and `Partial` compensations must test the receipt sum against the captured target (equals; positive and below), but `recover` uses `size(receipts)` (`<= 5`; `> 0 and < 5`) and `targetFull`/`targetPartial` are captured and never read. FR-517 says a recipe element the spine refuses is a ticket and never a deletion from R; the FR-517 note records only the `Workflow` to `Attempt`/`Recovery` type change. | examples/checked-handoff/unit-event-position.native:30-65 | implementation-bug-despite-evidence |
| FND-004 | low | FR-517-AC-2 says sidecar clause ids, names, requirement entries, activation forms, profile selections and authority equal the in-memory checked unit; the test compares only each row's `binding.clock` and `clock_name` to the package. Names are compared to nothing (the package carries none) and the activation `trigger` is stored as the binder name only (the trigger's second tuple member is dropped in `Selection::derive`), not the trigger type FR-517 lists, and `check` never compares it. | tests/it/checked_handoff.rs:251; src/checked_handoff/selection.rs:derive | correct-requirement-no-evidence |
| FND-005 | low | FR-517-AC-3 test checks the two compensation nodes differ and share some dependency; it does not assert `Full` and `Partial` by key or that the shared dependency is the `Main::Applied` effect node. | tests/it/checked_handoff.rs:291 | correct-requirement-no-evidence |
| FND-006 | low | FR-517-AC-8 and AC-9 are text greps. AC-8 never compiles a downstream crate with only the feature on (the reviewer ran `clippy --features checked-handoff`, which does); AC-9 greps for two spellings while the module itself parses package bytes outside `read_checked_package` twice (`WireGraph::of_package_bytes` in `Selection::derive`, `serde_json::from_slice(base)` in `corpus::build`). FR-518-AC-8 counts `pub const` lines equal to 3, which breaks on any new documented constant. | tests/it/checked_handoff.rs:486,539,1049 | correct-requirement-no-evidence |
| FND-007 | low | `emit_checked_v2::compile` re-implements the qsl-replay recompile pipeline (parse, select, check, package) instead of calling `compile_package_with_temporal_selections_and_execution_points`: it calls `check_with_execution_points` without the temporal selections the replay path passes to the check stage, uses `SpineLimits::default()` rather than the request limits path, and drops `check_unit_owner` (unreachable here, because the unit label is `source:` and a library label `library:`). The existing 11 emit tests pass and the reader admits R, so no failure is shown; the risk is drift from the replay pipeline. | src/emit_checked_v2.rs:compile; qsl-replay/src/compile.rs:236 | implementation-bug-despite-evidence |
| FND-008 | low | Three tests assert less than their trace says. `the_definition_cases_move_onto_authority_and_the_unknown_profile_refusal` (FR-518-AC-4) only compares the test's own hand table; `the_extreme_rationals_are_package_values` (ignored, QSL-1522) body is `model.contains("Exact")`, which would pass for a package without the rationals; FR-518-AC-9 is traced only to an ignored test, so the matrix shows it tagged while no running test covers it. | tests/it/checked_handoff.rs:944,400,1068 | correct-requirement-no-evidence |
| FND-009 | low | `check_binding` returns axis `ClauseNode` for a `clock_name` difference, a missing or surplus binding row and a clock-kind change, so a consumer cannot tell them apart; `check_clause` skips the name test when a node has no `declaration`; `check_profile` takes an unused `_package`. | src/checked_handoff/selection.rs:check_clause,check_profile,check_binding | implementation-bug-despite-evidence |
| FND-010 | low | `tree::publish` checks `exists(directory)` and then `rename`s; on POSIX a rename onto an existing empty directory succeeds, so a racing creator is replaced instead of refused. The temporary-tree removal error is discarded with `let _ =`, and the sibling name carries the process id only. | src/checked_handoff/tree.rs:publish,sibling | implementation-bug-despite-evidence |


## New findings (disposition pass 1)

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-011 | medium | The code now follows a table the merged FR-518 does not state: rows 8 and 13 are reader cases (merged: selection), counts are 21 reader and 10 selection (merged: 19 and 12), and row 12's mutation moves clauses, lock selection and binding rows together (merged text: one profile identity differs). Merged FR-518-AC-2 and AC-3 and TC-1313 rows 2 to 4 therefore disagree with the tests until the separate spec PR lands. Merge order: spec PR first. | tests/it/checked_handoff.rs:table; src/checked_handoff/corpus.rs:definition_revision,declaration_name | spec-code mismatch pending spec PR |
| FND-012 | low | `SelectionAxis` gained four variants (`clock.kind`, `clock.name`, `clause.binding`, `package.nodes`) that the merged FR-518 Outputs axis list (twelve names) does not contain, so `Selection::check` can return an axis the spec never names; `clock.name`, `clause.binding` and `package.nodes` have no corpus case. Carry the four names into the spec PR. | src/checked_handoff/selection.rs:SelectionAxis; FR-518 Outputs | spec-code mismatch pending spec PR | missing-requirement |
| FND-013 | low | `tree::claim_and_move` creates the destination with `create_dir` and then renames the temporary tree onto that empty directory. POSIX allows it; on Windows renaming a directory onto an existing one fails, so `write` could never succeed there, and a crash between the two calls leaves an empty directory at the requested path that every later `write` refuses as `Exists`. | src/checked_handoff/tree.rs:claim_and_move | implementation-bug-despite-evidence |

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | New test `each_sidecar_value_is_compared_on_its_own_axis` edits each sidecar value against the unchanged admitted package; reviewer mutations on 8cdba1f10: identity comparison deleted gives 10 red, revision comparison deleted 1 red, write_with read-back deleted 1 red (`a_refusal_or_difference_writes_nothing`, now failing the comparison for real through a reader double), binding-count check deleted 1 red. `clause.name` stays unexercised: a temporal_clause node carries no declaration, shown by `the_clause_name_has_no_package_side_value`; the remaining question is a spec ruling (spec PR). |
| FND-002 | deferred | The full-list test is kept and ignored with reason `gated on QSL-1512 (PR 2140)`; the running test is renamed `the_directory_holds_the_outputs_table_except_the_inventory`. Tracked by QSL-1512. |
| FND-003 | fixed | FR-517 now records the departure with a ticket for each: size instead of receipt sum (QSL-1555), Attempt and Recovery records (QSL-1556), no CLI flag (QSL-1557), units (QSL-1558), rationals (QSL-1522); all four new tickets exist in Backlog. |
| FND-004 | still-open | low, non-blocking: names, authority, profile identity and revision, requirements and activation form are now compared (`the_written_package_is_admitted_and_matches_its_sidecar`); the trigger type is still not in the sidecar (binder name only) although FR-517 lists it. Either record it or amend FR-517 in the spec PR. |
| FND-005 | fixed | The AC-3 test now asserts `Full` and `Partial` in the node keys and that exactly one shared node is the `Main::Applied` run node. |
| FND-006 | still-open | low, non-blocking: no feature-only downstream compile test for FR-517-AC-8 and `FR-518-AC-8` still counts `pub const` lines; the AC-9 grep now counts the two permitted parses per file instead of two spellings. The reviewer ran `clippy --features checked-handoff` and `check --no-default-features`, both clean. |
| FND-007 | accepted-no-change | Documented on `compile`: the check stage runs `check_unit_owner` (qsl-replay lifecycle.rs:681) and a byte-for-byte comparison with `compile_package` exists in tests/it/emit_checked_v2.rs through `producer_bytes`; the check stage still gets no temporal selections because they are joined to checked clauses by name, which the doc states. |
| FND-008 | fixed | The AC-4 test now reads the cases from the written directory and asserts the axes and the `unknown_profile` refusal; the rational test (ignored, QSL-1522) asserts both rationals in the package; the AC-9 tag is removed from the ignored test so no running coverage is claimed. |
| FND-009 | fixed | Distinct axes `clock.name`, `clause.binding`, `clock.kind`, `package.nodes`; the unused `_package` parameter is gone; the silent pass without a `declaration` is commented as unreachable on a temporal clause node and shown by a test. The four new names are FND-012. |
| FND-010 | fixed | `claim_and_move` claims the destination with an atomic `create_dir`; `an_existing_directory_...` leaves an existing empty directory as it was; reviewer mutation removing the temp-tree cleanup still turns that test red. New residue is FND-013. |

## Review method

Applied /dev-team:reviewer with code-review and rust-review (rust-style baseline) as one lane in this file. Read every
changed file in full; ran the gates above; ran mutation experiments in the review clone and reverted them.
Rust checks: ownership and signatures (borrowed `CheckedPackageV2` surface, `Environment` seam), error enum with
stable `ErrorCode`, panic surface (none in production paths), lints (none allowed), feature gating (`cfg(feature)` on
`Compiled` fields and `pub mod` consistent; default and no-default builds compile), dependency surface (none added),
resource bounds (`CheckedPackageReadLimits::bounded()` for every read), filesystem effects (sibling temp then
rename, no partial tree), determinism (BTreeMap path order, pretty JSON with trailing newline).

## Verification

`git diff origin/main...HEAD`, the gate commands in Summary, and the two mutation runs. Clean units: FR-517-AC-5, AC-6
(mutation red), AC-7, AC-10; FR-518-AC-1 (with a double), AC-2, AC-3, AC-5, AC-6, AC-7.
