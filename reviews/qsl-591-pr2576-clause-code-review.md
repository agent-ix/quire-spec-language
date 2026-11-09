---
id: SR-164207
title: QSL-591 code-review PR2576 clause half
type: SpecReview
analysis: code-review
review_set: subset
scope: "agent-ix/quire-spec-language; QSL-591 PR2576 @69b23a179 (rebased on 73f1a9153) clause scope: e1248cd98 and 69b23a179 over qsl-eval/src/execution.rs, qsl-replay/src/spine/call.rs, qsl-replay/src/spine/call/execution_tests.rs, qsl-replay/src/spine/clause/tests/execute_clause.rs, qsl-replay/src/spine/lifecycle/tests.rs, Cargo.lock"
relationships:
- type: references
  target: ix://agent-ix/quire-spec-language/FR-279
- type: references
  target: ix://agent-ix/quire-spec-language/FR-294
- type: references
  target: ix://agent-ix/quire-spec-language/FR-296
---

## Summary

Ticket: QSL-591. Code review with the rust-review lane folded in (rust-style loaded) for the clause scope of PR #2576 at 69b23a179. It was checked against the merged spec text from #2580 (73f1a9153): FR-279, FR-294, FR-296 and FR-106. Earlier rounds (SR-164205, SR-164206) closed the function path. This pass covers what is new:

- `EntrySelection::Clause(QualifiedName)` and `EntrySelection::name`.
- `ExecuteSelection::Function { name, arguments, environment }` and `ExecuteSelection::Clause { clause, input: AdmittedObservations }`, with `environment` removed from `ExecuteRequest`.
- `PreparedEntries::call` keyed by `&QualifiedName`, and the new `PreparedEntries::evaluate(clause, input, &ScalarLimits, cancel)`.
- The interpreter's `evaluate` over `evaluate_clause`, returning its `evaluation` member, and `execute` dispatching per selection without reading `supplied` for a clause.
- The `ExecuteLimits` `compile_fail` doctests (E0063, E0277) with a control.
- `Call::new` taking `ExecutionBackendChoice::default()`, and the quire-eval lock moving to a5c89a9 (`Default` derive).

No builds were run, per the brief.

## Verdict

PASS with one low spec-wording finding. Every signature matches the merged FR-279/FR-294 text. The interpreter's `evaluate` polls the handle before it runs and checks `tripped()` after, as `call` does. `run_clause` is left untouched, which is what FR-279 "Clause execution and `run_clause`" requires (one evaluator, two callers). The `#[allow(clippy::large_enum_variant, reason = "FR-279 states the clause input by value")]` is justified: FR-279 fixes `input: AdmittedObservations` by value, and `execute` takes `&ExecuteRequest`, so the large variant is never moved on the hot path. It follows the file's existing `#[allow(..., reason)]` convention. `Call::new` stopped being `const fn` because `Default::default()` is not const, which is fine before release.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | FR-279's new reason 3 for keeping `run_clause` off `execute` says "the spine is a replay path, which runs only through S6a (ADR-029 EB-3), so it never takes a chosen backend". Read literally, this contradicts `qsl_replay::spine::run` / `run_with_backends`, which take `Call.backend` and supplied backends, as FR-296 and FR-100 (native-run as the command encoding of `execute`) need. ADR-029 EB-3 restricts only replay. Narrow the sentence to `run_clause` / replay. This is spec text, for the spec lane. | spec/functional/FR-279-execute-a-checked-entry-as-a-library-operation.md:41-60 |

## Coverage

Examined every hunk of e1248cd98 and 69b23a179. The production code, `qsl-eval/src/execution.rs`, has no panics on caller data, no unsafe, and documents every new public item. `execute` looks up `unsupported` by full `EntrySelection` equality (variant plus name), so a function and a clause with the same name cannot be confused. The test stubs look up by name only, which is test scaffolding.

```yaml
scope:
- id: FR-279
  path: spec/functional/FR-279-execute-a-checked-entry-as-a-library-operation.md
  role: examined
  excerpt: "A clause selection evaluates one checked state clause over the observations FR-106 admitted for it: `execute` and `PreparedEntries::evaluate` take the clause's `QualifiedName` and that `AdmittedObservations` value, which are exactly the inputs of S6a's clause entry `CheckedPackageEvaluation::evaluate_clause` (FR-107)."
- id: FR-294
  path: spec/functional/FR-294-run-every-execution-backend-behind-one-seam.md
  role: examined
  excerpt: "`PreparedEntries::evaluate(&self, clause: &QualifiedName, input: &AdmittedObservations, accounting: &ScalarLimits, cancel: &Cancel)` has the contract of `CheckedPackageEvaluation::evaluate_clause` (FR-107) over `input`, metered by `accounting`, returning that `ClauseEvaluation`'s `evaluation` member."
- id: FR-296
  path: spec/functional/FR-296-select-the-execution-backend-in-the-request.md
  role: examined
  excerpt: "The choice is `ExecutionBackendChoice { Interpreter, Aot, Jit }`, the `backend` member of `ExecuteRequest` (FR-279), whose `Default` is `Interpreter`."
```
