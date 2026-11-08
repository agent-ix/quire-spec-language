---
id: SR-20261108210000000001
title: "base review of the clause execute input, run_clause relation and execute limits"
type: SpecReview
analysis: base
scope: "spec/functional/FR-279-execute-a-checked-entry-as-a-library-operation.md; spec/functional/FR-294-run-every-execution-backend-behind-one-seam.md; spec/functional/FR-277-bound-every-lifecycle-operation-by-caller-limits.md; spec/decisions/ADR-029-lifecycle-cli-provider-plugin-cache-codegen-boundaries.md; spec/test-cases/TC-760-execute-returns-what-checkedpackage-call-and-evaluate-return.md; spec/test-cases/TC-778-the-interpreter-implements-the-execution-seam-and-the-seam-admits.md"
review_set: base
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-279
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-294
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-277
    type: references
  - target: ix://agent-ix/quire-spec-language/ADR-029
    type: references
---

# Base review of the clause execute input, run_clause relation and execute limits

## Summary

QSL-591. One base-checklist pass over the branch diff against `origin/main`.
The diff defines the seam's clause input (FR-106 `AdmittedObservations`) and
function arguments (`Vec<Value>` / `&[Value]`), states that clause execution
and FR-109's `run_clause` share S6a's `evaluate_clause` with `run_clause` as
the reference pipeline, and adds FR-277's `supplied` to `ExecuteLimits` and
the new `CallLimits`, replacing `ExecuteLimits::default()` with explicit
supply of `ScalarLimits`. Grounding: `qsl-eval`'s `CheckedPackageEvaluation`
(`call_supplied`, `evaluate_clause`), `qsl-replay`'s `spine::clause`
(`run_clause`, `check_clause_admitted`), the QSL-591 code PR's
`qsl-eval/src/execution.rs`, and quire-exact's `ScalarLimits`, which has no
`Default`. Four findings, all fixed in the same branch. ID formats, links and
AC-to-TC coverage are clean; `make check-spec-validation` exits 0 with no
warning on a touched file. No hash, digest or pin appears.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | FR-279-AC-5 and TC-760 step 5 claimed "zero evaluation consumption" through `execute`, which builds its meter internally and exposes no consumption, so the claim was unverifiable there (FR-277 owns it at `call_supplied`). Fixed: the clause is removed; the AC asserts equality with `call_supplied`'s `CallFailure::Supplied`. | FR-279-AC-5; TC-760 | wrong-requirement |
| FND-002 | low | FR-277 Inputs' new explicit-supply exception named "the storage and output-byte fields FR-255 makes required" vaguely. Fixed: names `V2ReadLimits.storage` and `EmitLimits.output_bytes`. | FR-277 Inputs | wrong-requirement |
| FND-003 | low | ADR-029's operation table row for `execute` still listed "a `QualifiedName` or clause selection, arguments, `ObjectEnvironment`, accounting limits", which no longer matches FR-279's selection and limits. Fixed: the row names both selection shapes and the three limits. | ADR-029 operation table | wrong-requirement |
| FND-004 | low | FR-277's explicit-supply exception has no AC of its own; it is evidenced by FR-279-AC-5's `compile_fail` doctest and the existing FR-255-AC-15 and AC-20. Accepted as is: adding a duplicate AC would add no evidence. | FR-277; FR-279-AC-5; FR-255-AC-15; FR-255-AC-20 | correct-requirement-no-evidence |
