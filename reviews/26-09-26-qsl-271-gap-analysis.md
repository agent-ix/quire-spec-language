---
id: SR-675
title: "Spine run (FR-100) delivery gaps"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@c23b3128c6a16c60b2d30021ef87e9c8490b9109; FR-100; FR-026; TC-450; TC-451; TC-452; spec/tests.md; spec/spec.md; tests/it/spine_run.rs; qsl-replay/src/spine/call.rs; tools/arch-lint/api_surface.rs; src/command.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-100
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-450
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-451
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-452
    type: references
---
## Summary

Ticket: QSL-271. PR: quire-spec-language#468 at c23b3128. This review traces
FR-100-AC-1 to AC-9 through TC-450 to TC-452 to their tagged tests, and checks
QSpec FR-301's exit codes (origin/main) against the code.

The kernel-refusal deferral (FR-100's `Outcome::Refused` rows, TC-452 step 4's
kernel-refusal expectations, and `Refusal::code()`) is out of scope by
decision, and is not raised as a gap.

- The fallback is isolated in `qsl-replay/src/spine/call.rs`.
- FR-100 Status reads "Implemented under QSL-271, except the outcome mapping's kernel-refusal rows". That is partial in substance, although the word "partial" is not used.
- The TC-452 row in tests.md is 🚧 with the exception named.
- The TC-450 and TC-451 rows claim ✅ although steps are untested (below).

FR-026 still holds: `tests/it/standalone.rs` is unchanged and runs in the
gates. The `quire-extraction` branch's behaviour for `0-draft` is unchanged.

| AC | Tests | State |
| --- | --- | --- |
| FR-100-AC-1 | tc_450_step_1 (CLI) and tc_452_seven (unit) | covered |
| FR-100-AC-2 | tc_450_step_2 (weak oracle) and tc_450_step_3 | covered |
| FR-100-AC-3 | tc_450_step_4, tc_450_step_5 and malformed work_units | partial: no-call vacuous, no extraction case, step 6 absent |
| FR-100-AC-4 | tc_451 steps 1-4 (CLI and unit) | covered |
| FR-100-AC-5 | tc_451 steps 5-6 (CLI and unit) | covered |
| FR-100-AC-6 | tc_451_step_7 (CLI and unit) | covered |
| FR-100-AC-7 | qsl-replay unit tests plus TC-390 | covered |
| FR-100-AC-8 | arch-lint unit tests | tests present; check bypassable (SR-674 FND-001) |
| FR-100-AC-9 | tc_452_step_4 (convert_outcome only) | partial beyond the deferral: no renderer or exit assertion |

## Verdict

**Gaps found.** The implementation traces to FR-100, apart from the spec and
code disagreement on `clauses` (SR-674 FND-002). The test matrix overclaims
TC-450, and AC-3 and AC-9 are only partly backed.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | tests.md marks TC-450 "✅ Passed locally", but TC-450 step 6 (`libraries` plus `models`, exit 0 and completed) has no test, and step 4's "no call" and `extraction` items are not exercised (SR-674 FND-005, FND-007). | spec/tests.md:233; tests/it/spine_run.rs:137-217 |
| FND-002 | medium | FR-100-AC-9 / TC-452 step 4 expects each outcome's exit status via "the root crate's renderer and exit mapping". The test stops at `qsl_replay`'s `convert_outcome`, so the exit statuses 20/21 for undefined and family refused are untested, apart from the deferred kernel rows (SR-674 FND-006). | qsl-replay/src/spine/call.rs:703-799; src/command/output.rs:322-370 |
| FND-003 | medium | Spec and code disagree on `program.clauses`. FR-100 Inputs says a `1-draft` `program` carries no `clauses`, but the decoder requires the key, so a spec-conformant request is refused. Either the wire type or FR-100's Inputs must change (SR-674 FND-002, coder-reported gap 2). | spec/functional/FR-100-run-a-named-function-through-the-spine.md; src/command/wire.rs:167-168 |
| FND-004 | low | The TC-450, TC-451 and TC-452 files still say "Specified under QSL-271. Not run.", which contradicts tests.md's ✅ and 🚧 rows. | spec/test-cases/TC-450-cli-run-routes-a-program-by-its-declared-edition.md; spec/test-cases/TC-451-spine-run-binds-arguments-and-maps-outcomes-to-exit-codes.md; spec/test-cases/TC-452-spine-run-entry-lives-in-qsl-replay.md |
| FND-005 | low | FR-100 Status documents the kernel-refusal fallback as `RunRefusal::Fault` (`runtime_invariant`) but not its exit status, which is 20 via `Code::exit_code`. FR-100's internal-failure row says 30 (SR-674 FND-003). | spec/functional/FR-100-run-a-named-function-through-the-spine.md; qsl-replay/src/spine/call.rs:12-24 |
| FND-006 | low | The AC-8 check was added to `arch-lint api-surface` (coder-reported gap 3). That CLI target is not part of `make ci` and exits 2 without `CG_CLONE`. The only gate is the workspace unit test `tc_452_spine_surface_check_passes_over_qsl_replay`, which does run in `make ci`. That is acceptable, but the CLI arm is effectively never run. | tools/arch-lint/main.rs:273-296; Makefile:141 |

## Coverage

Every FR-100 AC has at least one tagged test (`#[trace("TC-45x", "FR-100-AC-n")]`).

No underspecified code was found. `NativeRunSelection::Missing{Selection,Snapshots,Invocations}` moves FR-026's required-member refusals from decode to after the edition read. It keeps the same code (`invalid-request`) and stage (`request`), and FR-026 owns it.

## Dispositions

Reviewer disposition pass at efa42552 (based on a643665a, #469).

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 8eda2b73 and 630728ba. TC-450 step 6 is tested, and step 4's no-call and extraction items are isolated (mutant killed). |
| FND-002 | deferred | partly fixed: 8eda2b73 renders `Undefined`. The refused and fault rows' renderer and exit mapping are still untested, so TC-452 ✅ overclaims (SR-674 FND-013). |
| FND-003 | fixed | 3f49b580. The decoder makes `clauses` optional, and FR-100 Inputs is unchanged and now satisfied. |
| FND-004 | fixed | 630728ba. The TC-450/451/452 Status sections say "Passed locally". |
| FND-005 | fixed | 630728ba. FR-100 Status is rewritten for the full mapping (kernel rows are live, exits 30 direct). |
| FND-006 | accepted-no-change | The AC-8 CLI arm stays outside `make ci`; the workspace unit tests gate it. |

## Dispositions, round 2 (46fff6e1)

Post-rebase SHAs for the round-1 dispositions above: 3f49b580→482a1ac1,
630728ba→239e664f and 8eda2b73→3025bc3e.

| FND | Outcome | fix_sha / reason |
| --- | --- | --- |
| FND-002 | fixed | 2a059b32. The CLI's renderer and exit mapping are tested for the record (exit by code, 22), family, kernel (20) and fault (30) rows; see SR-674 FND-013. The `locus` rendering gap is carried as SR-674 FND-019. |
