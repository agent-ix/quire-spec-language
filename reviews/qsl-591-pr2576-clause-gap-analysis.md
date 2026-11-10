---
id: SR-164208
title: QSL-591 gap-analysis PR2576 clause half
type: SpecReview
analysis: gap-analysis
review_set: subset
scope: "agent-ix/quire-spec-language; QSL-591 PR2576 @69b23a179 clause scope: FR-279-AC-2, FR-279-AC-5, FR-294-AC-1 (clause half), FR-294-AC-3/AC-4 (clause halves), FR-296-AC-4 (Default clause) against qsl-eval/src/execution.rs, qsl-replay/src/spine/clause/tests/execute_clause.rs and qsl-replay/src/spine/call/execution_tests.rs"
relationships:
- type: references
  target: ix://agent-ix/quire-spec-language/FR-279
- type: references
  target: ix://agent-ix/quire-spec-language/FR-294
- type: references
  target: ix://agent-ix/quire-spec-language/FR-296
---

## Summary

Ticket: QSL-591. Planless gap analysis of the clause scope against the merged #2580 spec text. Trace tags were collected by grep over `#[trace(...)]`.

New tagged tests:
- `execute_a_clause_returns_what_evaluate_clause_returns` (FR-279-AC-2)
- `supplied_limits_reach_the_function_call` (FR-279-AC-5, runtime half; the `compile_fail` half is the two `ExecuteLimits` doctests with a control in `qsl_eval::execution`)
- `the_interpreter_evaluates_a_prepared_clause` (FR-294-AC-1, clause half)
- `a_cancelled_handle_stops_evaluate` (FR-294-AC-4, clause half)
- `a_clause_the_backend_lists_unsupported_is_returned` (FR-294-AC-3, extra clause coverage)
- `the_default_backend_choice_is_the_interpreter` (FR-296-AC-4, `Default` clause)

## Verdict

PASS. Each clause AC is evidenced by a real call:
- **AC-2:** `execute` with a clause selection over FR-106-admitted observations from the healthy-parent (root 1, child 2) and violating-parent (root 3, child 2) snapshots equals `evaluate_clause`'s `evaluation`, completes true and false, and agrees with `run_clause`'s `Evaluate` disposition for the same request.
- **FR-294-AC-1 clause half:** prepares `ParentOrder` and `NoCycle`, matches `evaluate_clause`, and returns `ObservationsMismatch` for `NoCycle` observations.
- **AC-5:** `SuppliedRun::new` accepts zero budgets, so the expected value really comes from `call_supplied`.
- **FR-296-AC-4:** both clauses are now evidenced.

No stubs or coverage inflation. Plan completion: not assessed.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Coverage

```yaml
scope:
- {id: FR-279-AC-2, path: spec/functional/FR-279-execute-a-checked-entry-as-a-library-operation.md, role: examined, excerpt: "Over FR-109's ConfigVersion unit, for the FR-109-AC-1 healthy-parent and violating-parent requests, `execute` with the interpreter and a clause selection naming `ParentOrder`, with the `AdmittedObservations` FR-106 admits for it from the request's snapshot, returns `Ok` with the `evaluation` member of `CheckedPackageEvaluation::evaluate_clause`"}
- {id: FR-279-AC-5, path: spec/functional/FR-279-execute-a-checked-entry-as-a-library-operation.md, role: examined, excerpt: "A `compile_fail` doctest that builds `ExecuteLimits` with `..Default::default()` and no `accounting` fails to compile, and a control that states `accounting` explicitly with `SuppliedLimits::default()` and `PrepareLimits::default()` compiles."}
- {id: FR-294-AC-1, path: spec/functional/FR-294-run-every-execution-backend-behind-one-seam.md, role: examined, excerpt: "Prepared over FR-109's ConfigVersion unit with `ParentOrder` and `NoCycle` selected, `evaluate` of `ParentOrder` over the observations FR-106 admits from the healthy-parent and the violating-parent snapshots returns the `evaluation` member of `CheckedPackageEvaluation::evaluate_clause`"}
- {id: FR-294-AC-3, path: spec/functional/FR-294-run-every-execution-backend-behind-one-seam.md, role: examined, excerpt: "A test backend standing in for `jit` that cannot run the third of three selected entries returns `Ok` from `prepare`"}
- {id: FR-294-AC-4, path: spec/functional/FR-294-run-every-execution-backend-behind-one-seam.md, role: examined, excerpt: "`prepare`, `call` and `evaluate` with a handle already cancelled return `PrepareFailure::Cancelled`, `CallFailure::Cancelled` and `CallFailure::Cancelled`."}
- {id: FR-296-AC-4, path: spec/functional/FR-296-select-the-execution-backend-in-the-request.md, role: examined, excerpt: "`ExecutionBackendChoice::default()` is `Interpreter`; a test JIT passed as `SuppliedBackends { aot: None, jit: Some(&jit) }` through the blanket `SuppliedBackend` implementation runs `seven` when `jit` is chosen"}
```
