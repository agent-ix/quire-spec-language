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
| FND-004 | medium | Disposition pass, at 8c786c48. The new kernel spelling table does not cover every variant, so the "one total mapping" is not total, and it does not match the kernel's own spellings. Checked against every variant in quire-exact/src/outcome.rs: all four `Undefined` variants (`DivisionByZero`, `IeeeNotFinite`, `EmptyReduction`, `NoneValue`; :94-105) are present and correct, and no invented variant appears. But four of the thirteen `Refusal` variants (:109-164) are missing: `IeeeRationalOutOfDomain`, `ForeignReference`, `CardinalityOutOfBound` and `CheckedInvariant`. TC-452 step 4 says "each of the nine kernel refusals", which compounds the gap. There are two further conflicts. (a) The kernel already defines the language's closed `refused { code }` spelling for four variants, and it is snake case: `Refusal::code()` gives `ieee_nan_payload_not_representable`, `ieee_rational_out_of_domain`, `foreign_reference` and `cardinality_out_of_bound` (:169-184). FR-100 tables `ieee-nan-payload-not-representable` in kebab case, so the two spellings of one refusal disagree. `CardinalityOutOfBound` also has its own `cause()` tag, `below-minimum` or `above-maximum` (:188-190). (b) `CheckedInvariant` is "unreachable for an admitted program" (:161-163), so it is an internal fault (`runtime_invariant`, an internal failure) and not refused runtime input with exit 20. Failure scenario: the implementer's exhaustive match on `Refusal` meets four variants with no tabled spelling and invents one, or a `_` arm, which ADR-011 §5 forbids. Or AC-9 passes over nine variants while a cardinality refusal renders with an unspecified cause. Table all thirteen. Reuse `Refusal::code()` where it is `Some`, or say why the wire uses a different spelling. Map `CheckedInvariant` to the internal-fault path. | spec/functional/FR-100-run-a-named-function-through-the-spine.md:113, 119-139, 242; spec/test-cases/TC-452-spine-run-entry-lives-in-qsl-replay.md:37-38; quire-exact/src/outcome.rs:109-190 |

## Dispositions

Disposition pass at `agent-ix/quire-spec-language@8c786c48fcd7b6fdf4bc56799b4bf9db1fa2ce48` (fix commit `8c786c48`, "QSL-271 spec: fix SR-646 to SR-649 findings on spine run"). Each outcome was re-checked against the spec at that head, not taken from the commit message.

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed 8c786c48 | Spellings and a catalog map are now stated: kernel refusals map to `invalid_runtime_input`/20 with a kebab `cause`, and kernel undefined reasons are kebab spellings (FR-100:109-139). The table is incomplete and conflicts with `Refusal::code()`; see new FND-004. |
| FND-002 | fixed 8c786c48 | `function` is now segments separated by `.` with an identifier rule (FR-100:56-59). The empty string, an empty segment, a non-identifier and a multi-segment name each refuse `missing_declaration` at `call` with `details` `{"function"}`. AC-5 and TC-451 step 5 test `""`, `seven.`, `7x` and `module.seven`. |
| FND-003 | fixed 8c786c48 | `work_units` is now a JSON integer from 0 to `u64::MAX`. Anything else refuses `invalid-request` at `request` (refusal table and Behavior), and AC-3 and TC-450 step 4 test `18446744073709551616`, `-1` and `1.5`. |
| FND-004 | fixed in 96c921cf | FR-100 lists all thirteen kernel `Refusal` variants, each `cause` spelled by the kernel's own `Refusal::code()` (snake case), and Status records that `code()` gains arms for the nine it lacks. `CardinalityOutOfBound` carries its `below-minimum`/`above-maximum` tag in `details.violation`. `CheckedInvariant` maps to `runtime_invariant`, exit 30 (QSpec FR-301 tool failure). `Undefined` keeps kebab spellings because the kernel has no spelling method for it. TC-452 step 4 constructs all thirteen. |
