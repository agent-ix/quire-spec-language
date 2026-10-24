---
id: SR-15850
title: "Spec review — NativeIntegerCapacityBreach received at S6a"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language; spec/functional/FR-096-stage-limits-refusal-records-and-readers-carry-a-locus.md, spec/test-cases/TC-918-received-kernel-invariant-provenance-survives-fault-mapping.md at 114d56d"
review_set: subset
evaluated_revision: "114d56da20e0ce3caae41d7f5a85309e69410d65"
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-096
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-918
    type: reviews
---

## Summary

Ticket: STD-320. PR quire-spec-language#1265 adds a received-kernel paragraph
and FR-096-AC-21 to FR-096 and a Native Integer Capacity Vector section to
TC-918. Checked against quire-exact 72b2542 (`src/native_integer.rs`,
`src/outcome.rs`) and QSL main 5c4a1da (`qsl-eval` stop conversion,
`qsl-foundation` `InternalFault` and `kernel_refusal_record`,
`qsl-semantics` received-cause fixture) and the QSpec native-diagnostics
catalog `runtime_invariant` row.

## Verdict

PASS with two low wording findings. The mapping claim is correct: the cause is
raised at exactly one kernel site, the generic CheckedInvariant route maps it
to the stated InternalFault, `kernel_refusal_record` returns none for it, no
QSL crate calls the native integer evaluator, and the vector is constructible
and can fail. Ids are unique across open PRs; `make check-spec-validation`
passes with no FR-096 or TC-918 diagnostics.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The paragraph says the capacity interval is derived "at admission", but `NativeIntegerPlan::new` derives it at plan construction, which the same paragraph later distinguishes from operand admission. | spec/functional/FR-096-stage-limits-refusal-records-and-readers-carry-a-locus.md:366-370 |
| FND-002 | low | The cause is also raised when the native carrier computation itself overflows (`calculate()` returns `None`), where no value is produced; "produced a value outside the capacity interval" reads narrower than the kernel site. | spec/functional/FR-096-stage-limits-refusal-records-and-readers-carry-a-locus.md:366-370 |
