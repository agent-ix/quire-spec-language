---
id: SR-12061
title: "QSL-503 PR #929 code review: union construction, case evaluation and witness-union conversion"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@c2fb6d92d5eb6d5c5ea3f7852163f53dbba96db6; PR #929: qsl-eval/src/value/expression/evaluate.rs, qsl-eval/src/value/expression/evaluate/union_case.rs, qsl-eval/src/value/expression/mod.rs, qsl-eval/src/value/expression/union_admission.rs, qsl-replay/src/execute/argument.rs, qsl-replay/src/execute/tests.rs, qsl-replay/src/execute/tests/union.rs, qsl-semantics/src/check/ir.rs, qsl-semantics/src/check/mod.rs, qsl-semantics/src/check/union_fixture.rs"
review_set: subset
---

## Summary

Ticket: QSL-503. PR quire-spec-language#929, diff `origin/main...c2fb6d92d`
(10 files, +1428/-22). Code review with the rust-review lane folded in.
Read-only: no build or test was run; the author's full gate (EXIT=0 on
3517cc91f) is taken as given per the brief, and gates are not reviewed.

Checked hard, per the brief:

1. Skipped re-admission. The evaluator's `UnionValue` arm resolves the member
   and checks arity but does not re-admit payloads. This matches the
   evaluator's existing invariant: a checked body proves each payload against
   its position (FR-317). QSV's own `admitted` helper is documented as
   guarding values "a checked program guarantees is a member". The skip is
   needed: QSV's membership walk descends into union payloads
   (`membership.rs` union arm schedules every position), unlike records,
   where the walk stops at `composite_conforms`. So per-construction
   re-admission really would be quadratic. In replay, the option-payload and
   collection-element `admit` is skipped only for `Value::Union`. That is
   sound because the conversion is constructive. The union arm checks the
   wire's settled key equals the slot union's `union_key`, resolves the
   member by name among sealed members, checks arity, and converts each
   component against its declared position type. Every production caller of
   `convert_arguments` then re-admits the whole argument:
   `execute.rs` and `value_parity.rs` through `call_supplied`, and
   `composite_parity.rs` through `run.admit(SuppliedPhase::Admission, ...)`.
   So no value reaches the evaluator without admission.
2. Charges. Construction is one `retain_composite` after `pop_many`, giving one
   `composite.result-retain` with the value's occurrences. `select_arm` and
   `bind` charge nothing. The AC-3 test shows this by equality of the charge
   sequence with the construction and the body run separately, plus the
   `total - 1` / `total` boundary.
3. `case-arm-unmatched`, `case-scrutinee-member-unresolved` and
   `case-arm-binder-arity` are `invariant(..)` halts, which become an S6a
   `InternalFault`. FR-322 requires exactly that ("never a refusal").
4. `CheckedBody::from_nodes` / `id_at` are `#[cfg(any(test, feature =
   "test-support"))]`. `test-support` is enabled only from
   `[dev-dependencies]` (root, qsl-eval, qsl-replay), so with resolver 2 it
   never reaches a shipped lib, the same pattern as `depth_forms`. They mint
   `NodeId` positions, not node keys, inside `check/`.
5. `convert_arguments_in` is `pub(super)` and is the body `convert_arguments`
   now delegates to. It adds no public API, and the conversion semantics are
   unchanged for package callers.
6. Depth. Evaluation stays on the explicit `Task` stack (`Select` is a new
   task, not recursion). Conversion is the existing `pending` worklist. The
   replay 10,000-level test runs on a 512 KiB thread.
7. The new `Step::Union` variant is private to `argument.rs`. `Task::Select`
   is added to the task-to-node map. No match in `refinement.rs` or the other
   replay matches over `Value::Union` changed. Exhaustiveness is
   compiler-checked.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Verdict

PASS. No major bug found in the changed code. Union construction, `case`
selection and positional binding, the zero-cost selection, the S6a fault
classification and the replay union conversion are correct. The test-only
seams are gated correctly.

Not raised (below the brief's major-only bar, recorded for the planner):

- FR-322 Behavior says construction and `case` are reached "through one
  `Machine::apply` arm per node kind, each making one call into the `SumCase`
  evaluation module". The code evaluates `case` through `eval` and
  `Task::Select`, constructs inline in `apply`, and has no `SumCase` module.
  Behaviour is unaffected. Either the spec wording or the structure should be
  aligned when QSL-504 lands.
- QSV exposes `TypeEnvironment::evaluate_union`/`union`, which re-admit every
  payload. The evaluator does not use them (deliberately, see item 1). That
  is a divergence from how `tuple`/`record` construction calls QSV.
