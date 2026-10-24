---
id: SR-13351
title: "Code review of quire-spec-language PR #1047: QSL-530 E9 (4/4), strong-would-exclude hint (FR-133)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@9ae691752; PR #1047 (git diff refs/remotes/origin/main...HEAD, merge base 28fce249e = #1045): qsl-replay/src/model_check/replay.rs, qsl-replay/src/model_check/settle.rs, qsl-replay/src/lib.rs, qsl-eval/src/simulation/model.rs, qsl-analyze/tests/it/{strong_hint,example_units,lasso_replay,model_settle,units,main}.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-133
    type: reviews
---
# SR-13351: Code review of quire-spec-language PR #1047

## Summary

Ticket: QSL-530 (E9, PR 4 of 4). Code review with the rust-review lane
folded in. The PR adds `StrongFairnessHint` to `replay_model_trace`'s
result and copies it onto `ModelCheckRecord::hints` for a reproduced
refutation. It splits the old `loop_fairness` into `loop_states_of`, which
records the enabled identities and the taken one at each loop state, and
`unfair_constraint`, the FA-3 check. A new `strong_hints` reads the same
loop states. `ModelSystem::operations()` in `qsl-eval` lists the operations
for the walk.

Checks run at 9ae691752 in a fresh clone, with my own `CARGO_TARGET_DIR`
(not `make ci`):

- `cargo fmt --all -- --check`: clean.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean.
- `cargo test -p qsl-eval -p qsl-replay -p qsl-analyze`: all pass
  (qsl-analyze `it` 178, qsl-eval 85 unit + 587 `it`, qsl-replay 717 unit
  and its integration suites). The three `strong_hint` tests pass.
- `cargo test --test it` (root): 975 passed, 6 ignored.

**Rebuild check (the cherry-pick that took this branch's side).** The scope
diff is taken against merge base 28fce249e, which is #1045 merged on top of
#1044. A dropped piece of #1044 or #1045 would show there as removed lines.
It does not. The only removed function is `loop_fairness`, split into
`loop_states_of` and `unfair_constraint` with the same FA-3 logic. The other
removed lines are signatures that became `pub(crate)` and the `request(...)`
calls that now take the document and snapshot iterator. No `#[test]` or
`#[trace]` line is removed. `evaluate_open_prefix`, `PrefixShort`,
`NO_ATOMS`, the `letters: &dyn ModelLetters` argument and `UnfairLasso`
appear the same number of times at the merge base and at HEAD. Nothing
merged before the base was reverted.

**Hint soundness.** A lasso repeats its loop forever. So an operation (or
identity) enabled at a loop state is enabled infinitely often, and if no
loop step takes it, it is never taken. Strong fairness on it, `whole` or
`each` on that identity, therefore excludes the lasso. `strong_hints` names
an operation only under exactly those two conditions, from the enabled sets
replay recomputed. A stutter loop records no state, and a finite prefix
records none, so neither can be named. `Each` picks `.min()` over
`ModelTransition`, whose `Ord` is the canonical-bytes order
(quire-semantic-value), which is canonical transition order.
`first_enabled_position` is the trace index of the first loop state that
enables it. A hint is computed only on the reproduced arm. The record copies
it only on `Reproduced`, and the value, label, basis and counterexample are
unchanged. Hints that do not exclude the lasso were not found.

**Fair and unfair lassos.** An unfair lasso still refuses `UnfairLasso`
before any hint is computed. A strong clause still refuses `StrongFairness`,
since `unfair_constraint` checks first (see FND-002 on the ordering). There
is no compatibility layer. The new comments are at the density of the
surrounding code and cite FR-133 and ADR-019 SV-6.

## Verdict

**Not mergeable as it stands: FND-001 is a merge blocker.** The FR-133 code
itself is sound and the gates are green at this head. But the branch is
behind main by #1058 (FR-169 settlement map) and #1060. GitHub reports it
`CONFLICTING`, and `git merge-tree` against 4ee645632 conflicts in
`settle.rs` and `tests/it/main.rs`. Resolving it the way the rebuild did,
by taking this branch's side, would revert FR-169's rewrite of
`settle_model_check`. FND-002 and FND-003 are low. Coverage gaps are in
SR-13352, the gap analysis.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The PR cannot merge into current main (4ee645632). GitHub reports `mergeable: CONFLICTING` / `DIRTY`, and `git merge-tree --write-tree origin/main HEAD` conflicts in `qsl-replay/src/model_check/settle.rs` (two hunks) and `qsl-analyze/tests/it/main.rs`. #1058 (QSL-538, FR-169) replaced the `record`/`inconclusive`/`incomplete`/`failed` closures with a `Row`-based settlement (`let row = match outcome`, `Row::refuted(SettlementBasis::DecisiveCounterexample)`, `counterexample = Some(payload...)`). This branch edits the old closure form (adds `hints: Vec::new()` to `record` and `hints` to the `Reproduced` arm). Taking this branch's side, as the rebuild did for its own conflicts, would silently revert FR-169's settlement map. Fix: rebase onto current main, carry the hints through the `Row` form (set `hints` alongside `counterexample` on the reproduced arm), keep both `mod` lines in `tests/it/main.rs`, and re-run the gates. | qsl-replay/src/model_check/settle.rs:117-123, 162, 314-321 |
| FND-002 | low | `check_formula` now calls `loop_states_of`, which re-expands every loop state, before `unfair_constraint` makes the `StrongFairness` refusal. The old `loop_fairness` refused a strong clause before any expansion. Today the two agree: `settle()` has already expanded each position, and the successor relation is deterministic, so the `None` branch cannot fire there. But the refusal now depends on an expansion succeeding, and a strong clause pays for a full loop re-expansion it then throws away. The loop states are also a second expansion of states `settle()` already expanded. Fix: make the strong-kind check before `loop_states_of` (or keep the enabled sets from `settle()`'s own walk), so the refusal for unbuilt strong fairness stays first. | qsl-replay/src/model_check/replay.rs:683-692, 819-831 |
| FND-003 | low | The hint order is `ModelSystem::operations()`'s order: types by `EffectiveId` (a `BTreeMap`), then operation names alphabetically (`applications_of` collects names in a `BTreeSet`). The doc comment calls this "each type's operations in its effective view's order". FR-133 says "canonical declaration order", which no spec defines. An inherited operation is listed under the first admitting type by id, not its declaring type. No test checks the order: the two-hint `Counter` case asserts only `hints.len() == 2` and filters out the `Whole` one. Fix: name the order in the doc (identity, then name), and assert `inc` before `setTo` in the two-hint case. Ask the spec owner to define "canonical declaration order" (for example, by operation declaration identity). | qsl-eval/src/simulation/model.rs:596-615; qsl-analyze/tests/it/strong_hint.rs:186-200 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | 5bbe8d1b1 merges main (4ee645632) into the branch. The merge base is now main's head, and `git merge-tree` against main is clean. `settle_model_check` keeps #1058's `Row` form: `let mut hints = Vec::new();`, `hints = replayed_hints;` on the reproduced arm beside `counterexample`, and `hints` on the returned record. `tests/it/main.rs` keeps `mod state_graph_settle;` and adds `mod strong_hint;`. The diff against main removes nothing from #1058 or #1060. Its only removed lines are `loop_fairness`'s refactor, signatures that became `pub(crate)`, and the `bounded` helper hoisted to `lasso_replay::bounded_fixture`. |
| FND-002 | fixed | `check_formula` refuses a strong clause (`ModelTraceRefusal::StrongFairness`) before `loop_states_of` reads any expansion. `unfair_constraint` no longer carries the check. The refusal is again the first thing on the lasso path, as before this PR. |
| FND-003 | fixed | `hints_come_in_the_models_operation_order` asserts `["inc", "setTo"]` over `Counter`'s `setTo` variant. The test's doc names the order (types by effective identity, then each view's operations). Defining "canonical declaration order" in FR-133 stays with the spec owner. |
