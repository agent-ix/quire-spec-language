---
id: SR-13720
title: "Spec review of PR #1169 (FR-397 IEEE profile selection), integrity"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language@d8477bd126f11395bd4ad28120bcdcd063bd5908; git diff origin/main...HEAD (PR #1169): spec/functional/FR-397-select-the-ieee-profile-of-a-package-that-holds-an-ieee-operation.md, spec/test-cases/TC-1091-a-package-that-holds-an-ieee-operation-selects-the-ieee-profile.md, spec/functional/FR-091-produce-value-forms-and-assemble-package-declarations.md, spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md, spec/spec.md; read against agent-ix/quire-specification@014f0ce (FR-148, FR-201-AC-9, proposals/quire-v1/definitions/value-complete.md, complete-value-lock.json, shared-grammar.md, tests/checked_package_v2.rs); ticket QSL-948"
review_set: subset
date: "2026-10-21"
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-397
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-1091
    type: reviews
---
# SR-13720: Spec review of PR #1169, integrity

## Summary

Ticket: QSL-948. Integrity of FR-397 (AC-1 to AC-6), TC-1091 and the FR-091,
FR-093 and spec index edits, checked for faithfulness to QSpec FR-148,
FR-201-AC-9 and the complete-value lock's `package_selection` (conditional
slot `ieee_operation` → `ieee_profile`), and for whether each AC example
compiles or refuses as stated. Every AC source was compiled through
`qsl_replay::compile_admitted` on a scratch copy of the PR head (results in
SR-13722). Gates: `make check-spec-validation` exit 0 with no diagnostic on any
changed file; `tools/check-index-completeness.sh` passes. FR-397 and TC-1091
are unique across every origin branch.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-397's trigger is narrower than QSpec's. QSpec value-complete.md defines `ieee_operation` as a package that "declares or evaluates a `float32` or `float64` type, value or operation", and FR-148 says the checked package selects `quire.value.ieee754-2019-default/v1` and that its absence refuses semantic admission. FR-110 "Lock rows" delegates the conditional rows to QSpec's `package_selection` rules. FR-397 instead says that a declaration, parameter, return type or bit constructor alone is not an IEEE operation, and AC-2 requires `keep(a: Float64[nearest-even])` to select no `ieee_profile`. Such a package holds a float type with no IEEE profile, which QSpec says refuses admission. This is lead default 2. Either widen the trigger to any float type or value in the package (AC-2 then inverts), or get QSpec to narrow its trigger text first. | spec/functional/FR-397-select-the-ieee-profile-of-a-package-that-holds-an-ieee-operation.md:48-54; FR-397-AC-2; quire-specification proposals/quire-v1/definitions/value-complete.md:30-33; quire-specification spec/functional/expressions/FR-148-evaluate-ieee-floating-profiles.md (Behavior); spec/functional/FR-110-resolve-header-profile-selections-at-e3.md:168-175 |
| FND-002 | high | AC-2's second example cannot be built. `function one using v(): Float64[nearest-even] pure { float64(bits: 0x3ff0000000000000) }` refuses the whole unit at S2 with `UnrepresentedConstruct { production: FloatValue }`. FR-091 (lines 361-366) lists `float32(bits: ..)` and `float64(bits: ..)` as constructs no family represents. The unit never reaches a lock, so "also selects none" can be neither true nor false, and FR-397 Inputs wrongly lists the bit constructors as content of checked bodies. Either drop the example, or make FR-397 (or FR-091) give the bit constructors an S2 form and a checked node. Under FND-001's QSpec reading, that unit must then select the profile. | FR-397-AC-2; spec/functional/FR-397-select-the-ieee-profile-of-a-package-that-holds-an-ieee-operation.md:33-36; spec/functional/FR-091-produce-value-forms-and-assemble-package-declarations.md:361-366 |
| FND-003 | medium | The "no float comparison source form" default (lead default 1) ignores QSpec's comparison source forms. QSpec shared-grammar.md (lines 438-446) reserves the qualified intrinsics `quire::value::ieee::numericEqual(a,b)`, `totalOrder(a,b)`, `bitIdentical(a,b)`, `sqrt(x)` and `fma(a,b,c)` through the ordinary qualified-call production. checked-package-v2 catalogs `quire.op.ieee.numeric_equal`, `total_order` and `bit_identical`. FR-397 refuses only the six operators and says nothing about these calls; today `quire::value::ieee::numericEqual(a, b)` refuses as `MissingName`. Ticket acceptance 3 asks that the comparison question be settled. State what QSL V1 does with each reserved intrinsic call: either lower it or refuse it with a named code. | spec/functional/FR-397-select-the-ieee-profile-of-a-package-that-holds-an-ieee-operation.md:74-80; quire-specification proposals/quire-v1/shared-grammar.md:438-446; quire-specification tests/checked_package_v2.rs:6872-6876 |
| FND-004 | low | AC-6 restates an existing obligation. FR-090's internal-failure table already has the row "Evaluator IEEE arithmetic or IEEE-to-rational conversion: no admitted IEEE profile → `ieee-profile-unresolved`", and qsl-eval already probes it (invariants.rs:976, 993). Once FR-397 holds, no checked package with an IEEE operation lacks the profile, so the AC also leaves unsaid how its input is built (a hand-built evaluator input). Drop AC-6 and cite FR-090, or say the input bypasses the checker. | FR-397-AC-6; spec/functional/FR-090-return-a-family-outcome-or-a-typed-family-refusal.md:484; qsl-eval/src/value/expression/evaluate/invariants.rs:976-1000 |
| FND-005 | low | The sentence added to FR-093 ("The lock evidence for `ieee_profile` is the package selection FR-397 states.") sits unwrapped on the end of a wrapped line and names FR-397 with no link, while the matching FR-091 edit links it. | spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md:249 |

## Verdict

Changes requested: two high, one medium and two low findings.

Examined and clean:

- AC-1: the add sources parse and check up to the profile gate. Both refuse
  only `IeeeProfileNotAdmitted`, the gap this FR closes.
- AC-3: `convert<Rational[-100, 100; 1, 16]>(a)` already reaches
  `IeeeToRational` and refuses only `IeeeProfileNotAdmitted`, not the
  `TypeMismatch` the ticket reported.
- AC-4: the header `profile w` refuses `WrongSelectionRole` with and without
  an IEEE operation.
- AC-5: all six operators refuse `IllTyped(OperatorIneligible)` at the
  operator, and mixed widths refuse `IllTyped(TypeMismatch)`.
- The FR-091 sentence is consistent.
- The spec.md `contains` edge and index row are present.
- TC-1091 steps 1-6 map one-to-one onto AC-1 to AC-6.

Lead default 3 (ordering left to the code lane) adds no AC that depends on
selection order, so it leaves nothing unbuildable. Lead defaults 1 and 2 are
FND-003 and FND-001.
