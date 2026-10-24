---
id: SR-12046
title: "QSL-818 PR #922 code review: bind_scalar_operands against FR-367"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@efddc9649d53fa4849d7dcbe7056db8b8c272cd8; qsl-replay/src/spine/bind.rs, qsl-replay/src/spine/bind/tests.rs, qsl-replay/src/spine/call.rs, qsl-replay/src/spine/lifecycle.rs, qsl-replay/src/execute.rs, qsl-replay/src/execute/scalar_site.rs, qsl-replay/src/scalar.rs, qsl-replay/src/composite.rs, qsl-eval/src/value/expression/mod.rs, qsl-eval/src/value/mod.rs, qsl-package/src/emit.rs, qsl-package/src/lib.rs, Cargo.lock"
review_set: subset
---

## Summary

Ticket: QSL-818. PR quire-spec-language#922, diff `origin/main...HEAD` (15 files). This is a code review with the rust-review lane folded in. It was read-only: no build was run, because the author's `make ci` exited 0 before the final main merge.

The review covered these points. The parameter-node join (FR-098-AC-2, FR-092 shared nodes). The `InlineLiteral { node: Some }` literal read from the checked package against `{ node: None }`. The `OperandNotParameter`, `OperandNotArgument` and `OperandNotNative` value-only checks. The `Incomplete` outcome. Admission shared with `run` through `qsl_eval::value::admit_arguments`. The newly public helpers. `BindPackage::read`. `BindIdentityCause` against FR-357.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | `BindPackage::read` builds its Contract IR evidence from `emitted.evidence()`, which holds only required features, plus the domain package documents. It never supplies the dependency packages. IR's `admit_dependencies` refuses `MissingImport` for every lock `dependency_selections` entry that has no supplied package. So for any unit that imports a library, `read_checked_package` is not `AdmittedV2`, and binding faults with `RunRefusal::Fault("emitted-package-is-readable-by-contract-ir")`, although `run` accepts the same unit with its `DependencyInput`. `qsl_package::read_import_view` already supplies the closure (`ClosureReader::supply`). No test compiles a unit with a dependency. | qsl-replay/src/spine/bind.rs:60-84, qsl-package/src/emit.rs:1156-1163, qsl-package/src/checked_v2.rs:1007-1027 |
| FND-002 | medium | `value_occurrences_exceeded` builds a `CallIncomplete` that `run` never returns. FR-100's `run` charges no argument occurrences against `value_occurrences`, so `plus(4)` with `value_occurrences = 0` completes under `run`, but the binding returns `Incomplete`. The record is also built by hand: `charge_point: FunctionCall`, `consumed: 0`, `location: None`. FR-098's conversion reports the same limit as a conversion-phase `SuppliedStop`. It also counts converted nodes against `work_units`, which the binding skips (the step-7 test binds with `work_units = 0`). This contradicts FR-367-AC-8's "as `run` does" and Behavior 8's "no FR-100 behavior changes". The AC-8 test checks the locally built shape and never compares it with `run`, unlike the AC-3 and AC-4 tests. The spec is also inconsistent here: Behavior 7 cites FR-098's conversion, while AC-8 cites `run`. | qsl-replay/src/spine/bind.rs:331-333, qsl-replay/src/spine/bind.rs:375-392, qsl-replay/src/spine/bind/tests.rs:774-784, qsl-replay/src/execute/argument.rs:459-485 |

## Verdict

FIX, because of FND-001: a unit that imports a library cannot be bound, and the failure is an internal fault rather than a typed refusal. FND-002 needs either a code change (no `Incomplete` that `run` would not report) or a planner reconciliation of FR-367 Behavior 7 with AC-8.

The following are sound:

- **Parameter join.** It goes through `callable_parameter_keys` in declared order. A `let` or quantifier binder sits at the next level, so it cannot collide with one of the called function's own parameter nodes, and it refuses `OperandNotArgument` as specified.
- **Literal source order.** A literal graph node is classified before `parameter_declaration` is called. It maps to `InlineLiteral { node: Some }`, and to `GraphChild` in the AC-9 FR-357 check.
- **`i64` check.** It applies to the value only, and a range endpoint beyond `i64` binds.
- **Admission.** It reuses `bind_arguments`, `admit_model_arguments` and the former `validate` (now `admit_arguments`) under the call's `SuppliedRun`, as `run`'s S6a does. The AC-3 and AC-4 tests compare against `run` itself.
- **`locate`.** It follows FR-367 Behavior 2's cause order, filters occurrences by `Body { function }`, and handles shared content-addressed nodes.
- **Visibility changes.** They are crate-scoped except `checked_node_id`, `checked_occurrence` and `admit_arguments`, whose signatures expose no internals.

No unsafe code or new panics are introduced, and test `expect`s are confined to fixtures.

## Dispositions

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 3e4664769: `BindPackage::read` now takes the I2 reader limits (`SpineLimits::imports`) and calls the now-public `qsl_package::supply_closure` to supply the dependency closure before `read_checked_package`. `tc_1053_step_1_a_unit_importing_a_library_binds` binds `[4, 1]` over a unit that imports `test/geometry`, and `run` accepts the same unit. |
| FND-002 | fixed | 3e4664769: `value_occurrences_exceeded`, `Halt::Incomplete` and `BindOutcome::Incomplete` are removed, so binding charges nothing. FR-367's Outputs and Behavior 7 now say the binding reports no `Incomplete` (planner ruling option (a)), and AC-8 was amended in c749543de. No remaining code reads `BindOutcome::Incomplete`. |
