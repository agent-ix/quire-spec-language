---
id: SR-17012
title: "Gap analysis of PR #1446: tso flush fairness (FR-227, FR-228)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@2c2b756cb85b4a39dd51b30a50fabd915c7aa1b8; spec/functional/FR-227-carry-and-replay-a-weak-memory-counterexample.md, spec/functional/FR-228-derive-memory-fairness-constraints.md, spec/test-cases/TC-672-replay-a-weak-memory-counterexample.md, spec/test-cases/TC-673-memory-fairness.md, qsl-analyze/tests/it/tso_fairness.rs; ticket QSL-994"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-227
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-228
    type: reviews
---
# SR-17012: Gap analysis of PR #1446

## Summary

Ticket: QSL-994. Plan completion: not assessed. This review reads the computed matrix (`quire matrix --format tsv`, quire 0.36.2) at head 2c2b756cb:

- FR-227-AC-4 is tagged, at tso_fairness.rs:473.
- FR-228-AC-1, AC-2 and AC-4 stay untagged.
- No TC-only `#[trace("TC-673")]` tag and no doc comment raises an untagged AC to covered.
- No test doc comment names FR-228-AC-1, AC-2 or AC-4.

The ticket asks for four ACs: FR-228-AC-1 (tso half), AC-2 and AC-4, and FR-227-AC-4. Only FR-227-AC-4 is fully delivered.

FR-227-AC-4 binding, clause by clause:

- (a) "The lasso of FR-228-AC-2": the stem is `fork` and `w`'s store, and the loop is `r`'s load reading 0. The program swaps the guard "that load's binder reading 0" for `true`, because the litmus operations return no value. On the lasso the guard is true on every iteration either way, and the guard adds no step of its own: the loop is the `Lx` self-loop. So the lasso is the same.
- (b) "`w`'s buffer stays `[x := 1]` through `r`'s repeated load": the payload carries each step's reported memory component, and replay accepts the components before it judges fairness. The unflushed-lasso search requires a flush to be enabled at the loop state.
- (c) "refuses `invalid_runtime_input`/`invalid-value`": asserted.
- (d) "as unfair under `flush(w)`'s constraint": asserted as `UnfairProtocolLasso { constraint } == flush("w")`.

The binding is complete. The guard substitution should still be reconciled with FR-228-AC-2's text in spec PR #1442.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Three ACs in QSL-994's scope stay uncovered, so the ticket cannot close on this PR. FR-228-AC-1 lacks its ra half (QSL-933). FR-228-AC-2 has no `proved` verdict (FR-339's component certificate is not built for liveness) and no `refuted` verdict with the memory constraints ignored, because the engine has no switch to ignore them. FR-228-AC-4's TP-1 half is not asserted. No follow-up ticket is named for FR-339 liveness certificates or for the ignore switch or spec change. | spec/functional/FR-228-derive-memory-fairness-constraints.md:113-116, qsl-analyze/tests/it/tso_fairness.rs |

## Verdict

The PR's tested scope is sound and traced correctly: FR-227-AC-4 is complete. The tests for FR-228-AC-1, AC-2 and AC-4 are left untagged on purpose, which is the right call, and their doc comments carry no AC ids. To be mergeable, the lead keeps QSL-994 open or splits it. The untagged remainders need owners: QSL-933 for ra, a ticket for FR-339 liveness certificates, and #1442 for the AC-2/AC-4 "ignored" wording.

FR-227-AC-1 was already untagged on main. The reorder keeps its sc-replay refusal at the first differing step, and `a_tso_counterexample_replays_only_under_tso` passes.

## Dispositions

Disposition round 1, verified at head 33ee3220528f38283546795651ebed5f4c706dfd. The computed matrix (quire 0.36.2) shows FR-227-AC-4 and FR-228-AC-1, AC-2 and AC-4 untagged. The doc comment's "Not covered: FR-227-AC-4's `ra` clause" does not count as a binder.

After spec PR #1442, FR-227-AC-4 on main has a second sentence: an `ra` FR-228-AC-3 lasso refuses as unfair under the strong visibility constraint. `replay_refuses_the_lasso_that_never_flushes_w` asserts only the tso sentence, so it no longer completes the AC. Removing the FR-227-AC-4 tag (now `#[trace("TC-672")]` only) is correct.

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | deferred | QSL-994 stays open as the carrier. The PR body section "Not covered; QSL-994 stays open" lists FR-228-AC-1's ra half (QSL-933), FR-228-AC-2 (needs the FR-339 liveness certificate, and the "ignored" refutation has no switch), FR-228-AC-4's TP-1 half, and FR-227-AC-4's ra clause (QSL-933). No ticket is named yet for the FR-339 liveness certificate. |
