---
id: SR-1640004
title: "Code review (round 2) of PR 2291: FR-520-AC-11 per-binder charge log (QSL-1640)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@6bc6489f951546e8c26919f24a51d6fe07e3e7f9; QSL-1640; PR #2291; qsl-eval/src/value/expression/evaluate.rs, qsl-eval/src/value/expression/evaluate/invariants.rs, qsl-eval/src/value/expression/mod.rs, qsl-eval/src/value/expression/protocol_slot.rs, qsl-eval/src/value/expression/protocol_slot/charges.rs, qsl-eval/src/value/mod.rs, qsl-eval/tests/it/protocol_slots.rs, qsl-eval/tests/it/protocol_slots/acceptance.rs, qsl-eval/tests/it/protocol_slots/acceptance/state_compensation.rs, spec/functional/FR-520-*.md, spec/test-cases/TC-1315-*.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-520
    type: reviews
---
# Code review (round 2) of PR 2291

## Summary

Ticket: QSL-1640. Fresh code-review pass (rust-review lane folded in) over the full PR diff at head `6bc6489f9`, after the round-1 fixes recorded in SR-1640001. Earlier artifacts SR-1640001..003 are left as they are.

Checks performed:

- **quire-exact routing (IR-1280).** Binder accounting is routed through the quire-exact `Meter` interval API (`enable_charge_accounting`, `take_charge_accounting`, `disable_charge_accounting`, `ChargeAccounting::amount`, `LimitKind::ALL`) at quire-exact `87fefed`, the revision locked on main. QSL drains the interval once per machine step and attributes it to the step's owner. There is no local copy of the meter's interval mechanism. The `ChargeAmounts` sum/max combine is the QSL-side attribution layer, because `ChargeAccounting` exposes only `amount`.
- **accounting_version.** `BinderCharges::accounting_version()` returns `BINDER_ACCOUNTING_VERSION` (`qsl.protocol-slot.binder-charges/1`). Both `ProtocolSlotEvaluation` variants carry the log.
- **Cargo.lock.** The PR diff has no Cargo.lock or Cargo.toml change. The trial merge with origin/main `623cdcba6` also has no lock difference, so no first-party lock bump is present.
- **Gates on the trial merge.** `cargo check --locked --workspace --all-targets` is clean. `cargo clippy --workspace --all-targets -D warnings` is clean with default features and with `--all-features`. Focused tests: qsl-eval `--test it protocol_slots` passes 60 tests (6 ignored, all pre-existing). qsl-eval `--lib invariants` passes 31 tests.
- **Tests and size.** No test was weakened. The `protocol_slots.rs` and `acceptance.rs` edits reshape patterns for the new tuple field and extract `evaluate_gate`, and no assertion was dropped. New non-test code is about 520 lines, so a split is not needed. Nothing is vendored. QSpec is cited by ID only.

Two attribution defects were confirmed with a local scratch probe test, which was not committed and has been reverted:

- Body `target = 5 and not activated`: `activated` gets `[..,0,2,2]`, and `not activated` alone gets `[..,0,1,1]`. The `and` connective's own charge, over operands from `target` and `activated`, is therefore recorded under `activated`.
- Pure helpers `sqm(n, m) = if forall(x in three(n): x * 2 + m > 0) ...` (three elements) and `sqm1` (the same body over a one-element `one(n)`), called as `sqm(target, recovered.total)`: the `target` entry is `[5,0,0,0,0,0,0,4,7,5]` for three elements and `[5,0,0,0,0,0,0,2,7,3]` for one. Work for `target` does not grow with the element count. For comparison, the binder-free-body control `sq11`→`sq1` grows `target` work from 14 to 34. The `x * 2` of the second and later elements is lost to `unattributed`.

## Verdict

**Conditional.** The IR-1280 routing, `accounting_version`, conservation (entries combined equal the meter's consumed totals), capture restore and the call, dispatch and let propagation are correct and value-tested. Two attribution rules break the FR-520 outcome-map row and FR-520-AC-11 as this PR wrote them:

- A connective over two binders is charged to its right operand's binder.
- Query, `seq` and fold element slots take the merged owner of previous body results, not the source's owner.

One AC-11 test pins the first defect's wrong value.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The `and`/`or`/`implies` charge is attributed to the right operand's binder only. `Task::Short` pops the left operand and pushes `Task::Retain`, which drops the left owner. `Task::Retain` then pops only the right operand, so `retain_boolean` and the pushed result take the right binder. FR-520 says an operation over operands from two binders is made by no binder. Probe: `target = 5 and not activated` puts work 2 and result 2 on `activated`, while `not activated` alone puts 1/1 there. The test `an_operation_over_two_binders_is_unattributed_and_a_later_single_binder_operation_is_its_own` pins this value (`activated` `[..,2,2]`, message "`not activated` reads one binder after others were read"). Fix: carry the left operand's owner in `Task::Retain`, for example `Task::Retain(node, Owner)`, and absorb it in the Retain step. Then correct the pinned `activated` and unattributed values. | qsl-eval/src/value/expression/evaluate.rs:1776-1807; qsl-eval/tests/it/protocol_slots/acceptance/state_compensation.rs:817-830 |
| FND-002 | high | Iteration element slots take the merged owner of previous body results instead of the source's owner. `iterate` and `iterate_seq` absorb `iteration.owner`, pop the previous body result (merging its owner), then set `iteration.owner = self.step_owner()` and bind the element slot to it. After one body result over another binder, every later element is `Mixed`, so single-binder work on the element goes to `unattributed`. FR-520 says the element and accumulator bindings take the binder their source or bounds derive from. Probe: `forall(x in three(n): x * 2 + m > 0)` with `n = target` and `m = recovered.total` gives `target` work 7 for three elements and 7 for one element. Fix: keep the source/bounds owner (bind query and `seq` elements to it) separate from the results owner (the fold accumulator and the final result take the merge). | qsl-eval/src/value/expression/evaluate.rs:3471-3488; qsl-eval/src/value/expression/evaluate.rs:4014; qsl-eval/src/value/expression/evaluate.rs:4131; qsl-eval/src/value/expression/evaluate.rs:4151 |
| FND-003 | low | `Owners::step` is `Option<Owner>`, but `step()` maps `None` to `Owner::None` and `begin_step` sets `Some(Owner::None)`, so the two states are indistinguishable. A plain `Owner` field removes the `Option` and the `unwrap_or`. | qsl-eval/src/value/expression/protocol_slot/charges.rs:153 |

## Coverage

Examined:

- FR-520-AC-11, FR-520's `EvaluationReport` outcome-map row, and TC-1315 row 12.
- `charges.rs` in full.
- Every owner-propagation site in `evaluate.rs`: pop/push, frames, call, imported call, dispatch and guard, let, case binders, query, `seq`, fold, connective, branch, and the reader-root read.
- The meter capture save and restore in `mod.rs`.
- Every changed test.

FR-520-AC-9 was read as context.

## Dispositions

- FND-001: fixed. `Task::Retain` carries the left operand's owner and absorbs it; the pinned `activated` and unattributed values in the affected state_compensation tests were re-derived from FR-520 (a connective over two binders is made by no binder). New test `a_connective_over_two_binders_is_unattributed_and_leaves_the_right_binder_its_own_operation`.
- FND-002: fixed. `Iteration` and `SeqIteration` keep `source_owner` apart from the results `owner`; element slots bind to `source_owner`, the fold accumulator takes the merge. New test `every_element_of_a_query_over_one_binder_is_charged_to_that_binder`.
- FND-003: fixed. `Owners::step` is a plain `Owner`.
