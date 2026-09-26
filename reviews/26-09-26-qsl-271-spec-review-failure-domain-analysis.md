---
id: SR-649
title: "QSL-271 failure-domain review of spine run"
type: SpecReview
analysis: failure-domain
scope: "agent-ix/quire-spec-language@3c7c0a8bb365b9ce460dfda97a037682a5c94100; spec/functional/FR-100-run-a-named-function-through-the-spine.md; spec/test-cases/TC-450-cli-run-routes-a-program-by-its-declared-edition.md; spec/test-cases/TC-451-spine-run-binds-arguments-and-maps-outcomes-to-exit-codes.md; spec/test-cases/TC-452-spine-run-entry-lives-in-qsl-replay.md; code evidence quire-exact/src/outcome.rs, quire-exact/src/accounting.rs, qsl-foundation/src/diagnostic.rs, qsl-replay/src/identity.rs, qsl-replay/src/execute.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-100
    type: reviews
---

## Summary

Ticket: QSL-271 (PR agent-ix/quire-spec-language#458). This review looks for
failure modes FR-100 does not state, identity confusion and input edge cases.

Clean:
- The argument join covers an unknown name, a duplicate and a missing
  parameter.
- Values outside i64, non-integer values and a wrong kind each have a code
  and a stage.
- An absent `work_units` has a default.
- An `incomplete` result names its counter by the normative
  `quire.value.accounting/v1` member name (quire-exact/src/accounting.rs:99).
- The declaration order of refusals is total, before any model read.
- A compile refusal is carried unchanged.

Verdict: changes requested (two medium, one low).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Nothing defines how the outcome's `code` and `reason` strings are spelled for kernel results. `undefined.reason` is "<undefined reason>", which may be a kernel `Undefined` or a family `FamilyResult::Undefined`. Family reasons have catalog spellings (`UndefinedReason::as_str`: `precondition-false`, `absent-key`; qsl-foundation/src/diagnostic.rs:948-965). The kernel `quire_exact::Undefined` (`DivisionByZero`, `IeeeNotFinite`, `EmptyReduction`, `NoneValue`; quire-exact/src/outcome.rs:94-105) has no spelling in code or spec. Likewise `refused.code` is a "<catalog code>", but no mapping from a kernel `quire_exact::Refusal` (for example `InexactDecimal`, `RationalOutOfDomain`) to a catalog `Code` exists anywhere in the tree. The "20 or 21" exit status for a refused outcome also depends on that code. Failure scenario: the implementer invents spellings such as a Debug name or snake case, and the wire format `spine-run-result/1` fixes them with no spec behind them. TC-451 step 7's "naming division by zero" has nothing to compare against. State the kernel spellings, or a rule that produces them, and the kernel Refusal to catalog Code map with its exit class. | spec/functional/FR-100-run-a-named-function-through-the-spine.md:89-95, 146-153; spec/test-cases/TC-451-spine-run-binds-arguments-and-maps-outcomes-to-exit-codes.md:61-63 |
| FND-002 | medium | Nothing says how the `function` string becomes a name. It is "the called function's name, a string", and Behavior says "A name with more than one segment resolves to no function". No segment separator is defined (`.` as in AC-5's `module.seven`, or `::` as in source imports like `g::f`), and neither is the outcome for a string that is not an identifier. `qsl_replay`'s `QualifiedName` deliberately has no string constructor (qsl-replay/src/identity.rs:66-73, FR-071-AC-3), so run needs its own rule. Failure scenario: one implementation splits on `.`. Another treats `module.seven` as one segment that is not an identifier and refuses `invalid-request` at stage `request`, not `missing_declaration` at `call`. A third splits on `::`, so `a::b` passes. Each reading passes some of AC-5 and fails the rest, and `""` or `"7x"` has no stated outcome. Define the separator, or the single-identifier rule, and the code and stage for the empty string and for a non-identifier. | spec/functional/FR-100-run-a-named-function-through-the-spine.md:56, 131-133, 165 |
| FND-003 | low | `work_units` is "a non-negative integer" with no upper bound, and it is the `u64` S6a limit. A value above `u64::MAX`, such as `18446744073709551616`, has no stated outcome. The call-shape row of the refusal table covers only "a `call` that is not the closed object above" and argument values. Failure scenario: serde overflow surfaces as a decode error with an unspecified code or stage, or the value is saturated silently. State that a `work_units` outside the u64 range refuses `invalid-request` at stage `request`, and add it to TC-451 step 4. | spec/functional/FR-100-run-a-named-function-through-the-spine.md:64-67, 104-106 |
