---
id: SR-1735002
title: "gap analysis of PR #1735 (QSL-1181 slice 3b) against FR-375-AC-19/22, FR-322-AC-87/88/128/129, FR-341, FR-093"
type: SpecReview
analysis: gap-analysis
date: 2026-10-28
scope: "agent-ix/quire-spec-language; PR #1735 (7d6a76fdf); QSpec FR-375-AC-19, AC-22; FR-322-AC-87, AC-88, AC-128, AC-129 and its refinement step occurrences text; FR-341 (row binders as parameter nodes, planner ruling); FR-093"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-136
    type: reviews
---
# SR-1735002: gap analysis of PR #1735 (QSL-1181 slice 3b)

## Summary

Ticket: QSL-1181. Plan completion: not assessed.

| Requirement | Measured |
| --- | --- |
| FR-375-AC-22, FR-322-AC-128 | Met. `step_rows_carry_their_source_regions_by_canonical_ordinal`: under forward and reversed source order, the `steps`, node id and package id are equal, and each sidecar entry has its row's own region at its canonical ordinal. |
| FR-322-AC-88 | Met. The `generated` ordinals are 0 at the declaration and 1 and 2 at their rows' regions, and a row-free owner has ordinal 0 only. |
| FR-322-AC-129 | Met. An `any` owner keeps its sidecar and has no `generated` occurrence. |
| FR-322-AC-87 | Not exercised (FND-003). |
| FR-375-AC-19 | Partial, as the PR states. The declaration-naming half is tested. The memory-last half has no fixture until #1727. |
| FR-341 binders | Correct by probe (SR-1735001), but tested only at level 0 (FND-002). |
| Unwritten rows | History is tested. Whole population and `seq` are not (FND-001). |
| qsl-semantics test changes | Do not weaken coverage. The two gate tests keep the not-emitted refusal through a history row. The AC-12 subject tests now assert the stronger "checks and emittable". The source-order test strips only the `refinement row a..b` site strings, which move with source, and still compares everything else; `CheckedRow`'s `Debug` drops only `site` and `spans`. |
| source_node | The catalog node when there is a catalog, else the operation anchor, per the ruling. The IR model-root rule is IR-1077; `operation_rows_read_back` is ignored with its assertion kept. |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | **Two of the three refusal paths for unwritten rows are untested.** `rows_have_nodes` refuses a row containing `PopulationObjects`, `History` or `Seq`. Only a history row is tested (the two qsl-semantics gate tests). A whole-population field row and a `seq` row are not. If either check were wrong, lowering would hit a row form with no node, and the result would be a fault or a malformed package rather than `RefinementNotEmitted`. Add one emit test each, asserting `RefinementNotEmitted` naming the refinement. | qsl-semantics/src/check/refinement.rs (rows_have_nodes); qsl-semantics/src/check/mod.rs:4402-4422 |
| FND-002 | low | **No test has a row binder above level 0.** Every fixture operation has no parameters, so only `self` is ever bound. My probe (one Boolean parameter `flag`) confirms `flag` is a `parameter` at level 1. Add it as a test of FR-341's encoding. | qsl-package/src/emit/tests/admission_corpus.rs (rows_refinement) |
| FND-003 | low | **The FR-322-AC-87 tag does not match what the test checks.** AC-87 is about a node the compiler derives from a step row carrying `generated` at that row's index. QSL derives no node per step row; the rows are body members. The test checks the owner's ordinals, which is AC-88. Drop the AC-87 tag, or state that AC-87 does not apply to QSL's emission. | admission_corpus.rs (step_rows_carry_their_source_regions_by_canonical_ordinal trace) |

## Verdict

**Not mergeable until FND-001 is fixed** (two small tests). FND-002 and FND-003 can go in the same round.

Gates at 7d6a76fdf (locked-build, jobs 4, `--all-features`, debug info off, QSPEC_DIR=~/dev/quire-specification):
- fmt: exit 0.
- qsl-semantics: lib 804, it 1328, doc 30, all passing.
- qsl-package `refinement`/`catalog`/`protocol`/`rows`/`step`/`fairness`/`owner`: 84 pass, 4 ignored (IR-874 ×3, IR-1077).
- qsl-replay `refinement`: 43/43.
- `cargo clippy -p qsl-semantics -p qsl-package -p qsl-replay --all-targets -D warnings`: exit 0.

## Dispositions

Round 1 (head 2c563657d).

Round 1. Fix 7d6a76fdf..2c563657d (tests and the reviews/ SR files only).

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | `whole_population_and_seq_rows_are_not_emitted` refuses exactly `[RefinementNotEmitted{name: "R"}]` for a field row reading `count(s in C::config_history ...)` (whole population) and one using `seq(i in 0 .. 1: i)`. A mutant that makes `rows_have_nodes` always true fails it. |
| FND-002 | fixed | `a_row_parameter_above_self_binds_at_level_one` asserts the parameter nodes `self` at level 0 and `flag` at level 1 for `(self, flag)`, `(self, not flag)` and `(self, _)`. It asserts the encoding only: the read-back is the IR-1077-blocked `operation_rows_read_back`. That is acceptable, because FR-341's level encoding is QSL's to get right, and IR-1077 is about the anchor context, not the binders. |
| FND-003 | fixed | The FR-322-AC-87 tag is dropped from `step_rows_carry_their_source_regions_by_canonical_ordinal`, and its doc says AC-87 does not apply: QSL derives no node per step row. |

**"Memory rows last" read-back.** It was not recorded on QSL-1181, so I recorded it there as a completion item: it lands in the first branch holding both #1735's rows and #1727's `parallel`, with the assertion and source-order permutations spelled out.

**Verdict: mergeable** (stacked on #1723). No finding remains open on SR-1735001 or SR-1735002.

Gates at 2c563657d (locked-build, jobs 4, `--all-features`, debug info off):
- fmt: exit 0.
- qsl-package new and `refinement` tests: 7 pass, 1 ignored (IR-1077).
- `cargo clippy -p qsl-package --all-targets -D warnings`: exit 0.

