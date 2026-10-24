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

## New findings (disposition pass 1)

Re-checked at head 99d30b1cc9a4c10138f6abbcc6520035e258687c.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-006 | medium | AC-8 and TC-1091 step 8 name a construction that does not exist. They call for "a checked package built through the crate-internal package declarations with no `ieee_profile`". `PackageDeclarations::check` with no profile refuses every IEEE application as `IeeeProfileNotAdmitted` (qsl-semantics/src/check/check.rs:2731-2732, and :3370 for the conversion), so that package holds no IEEE node. Once FR-397 is built it selects the profile anyway. The construction qsl-eval's tests actually use is a fixture `CheckedBody::fixture` holding `NodeKind::Ieee` or `NodeKind::IeeeToRational`, applied by the `probe!` machine over `population_function_package()`, which has no IEEE profile (TC-917, `both_ieee_profile_checks_and_conversion_operand_check_are_real_producers`, FR-090-AC-15/16). Name that fixture construction, or replace AC-8 with a plain cross-reference to FR-090-AC-15 and TC-917. | spec/functional/FR-397-select-the-ieee-profile-of-a-package-that-declares-or-evaluates-a-float.md:109; spec/test-cases/TC-1091-a-package-that-declares-or-evaluates-a-float-selects-the-ieee-profile.md:32-33; qsl-eval/src/value/expression/evaluate/invariants.rs:64-70,966-995; qsl-semantics/src/check/check.rs:2731-2732 |
| FND-007 | low | "The reserved intrinsic identities ... have no source route in complete-V1" misstates QSpec. QSpec shared-grammar.md:438-446 gives these intrinsics the ordinary qualified-call source route under `quire.value.complete/v1`. The behaviour, refusing them as `missing_declaration`/`missing-name`, is the lead's V1 choice and is fine, but the sentence should say QSL admits no call to them, not that complete-V1 has no route. | spec/functional/FR-397-select-the-ieee-profile-of-a-package-that-declares-or-evaluates-a-float.md:89-91 |
| FND-008 | medium | Inputs says a float value comes from "a parameter, an IEEE arithmetic result or a conversion". FR-397 and FR-093 define only the float-to-`Rational` conversion, and that conversion yields a `Rational`, not a float. QSpec FR-148 defines explicit exact-to-IEEE conversion and width conversion, but no QSL row lowers either: `convert<Float64[nearest-even]>(r)` from a `Rational` or an `Int`, and `convert<Float64[nearest-even]>(f)` from a `Float32`, each refuse `IllTyped(TypeMismatch)` today (probe). State what QSL V1 does with these conversions (refuse `ill_typed`/`type-mismatch`, or lower them), and drop "or a conversion" from Inputs if no conversion yields a float. | spec/functional/FR-397-select-the-ieee-profile-of-a-package-that-declares-or-evaluates-a-float.md:40-44; quire-specification spec/functional/expressions/FR-148-evaluate-ieee-floating-profiles.md (explicit width and exact-to-IEEE conversion) |

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | 99d30b1cc: the trigger is now "A package raises `ieee_operation` when any unit of the package declares a `Float32[mode]` or `Float64[mode]` type (a parameter, a result or a field) or holds a checked IEEE arithmetic application ... or a checked conversion of a float to a `Rational`", matching QSpec value-complete.md. AC-2 now requires `keep` to select the row once. |
| FND-002 | fixed | 99d30b1cc: the bit-constructor example is gone. Inputs states that complete-V1 source has no float value literal and that S2 refuses `float32/float64(bits: ...)` as `UnrepresentedConstruct{FloatValue}` (FR-091). AC-2's second example is `plain` over `Int`, which compiles today (probe). |
| FND-003 | fixed | 99d30b1cc: the Reserved intrinsics bullet and AC-7 refuse each of `sqrt`, `fma`, `numericEqual`, `totalOrder` and `bitIdentical` as `missing_declaration`/`missing-name` naming the identity. The probe confirms all five refuse `MissingName("quire::value::ieee::<name>")`, which maps to `MissingDeclaration`/`missing-name` (refusal.rs:736, 2228). |
| FND-004 | still-open | AC-8 now says how its input is built, but the construction it names does not bypass the checker and cannot hold an IEEE node. FND-006 carries the fix; this finding closes with it. |
| FND-005 | fixed | 99d30b1cc: FR-093:249-251 now wraps the sentence and links [FR-397](FR-397-select-the-ieee-profile-of-a-package-that-declares-or-evaluates-a-float.md). |

## New findings (disposition pass 2)

Re-checked at head 7b9515e292481faae60a566a918bb9746142f24e.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-009 | low | The Strict admission bullet ends "... the arithmetic of a `Float32` or `Float64` function and the conversion above." The new "Conversion into a float" bullet now sits between it and the float-to-rational bullet, so "the conversion above" first reads as the refused conversion. Say "the float-to-rational conversion". | spec/functional/FR-397-select-the-ieee-profile-of-a-package-that-declares-or-evaluates-a-float.md:79-85 |

## Dispositions (round 2)

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-004 | fixed | 7b9515e29: the evaluator-failure obligation is now FR-397-AC-9, a cross-reference to FR-090-AC-15 through TC-917's `CheckedBody::fixture` construction. |
| FND-006 | fixed | 7b9515e29: AC-9 names the real construction. TC-917's test `both_ieee_profile_checks_and_conversion_operand_check_are_real_producers` (qsl-eval/src/value/expression/evaluate/invariants.rs:965-995) builds both an `Ieee` and an `IeeeToRational` fixture node and probes `ieee-profile-unresolved` for each. TC-1091 step 8 no longer builds a package. The TC-917 edit adds only a `verifies` edge to FR-397 and FR-397-AC-9 in its Scope; the existing steps and tags are unchanged, and the gates pass. The code lane still has to add `FR-397-AC-9` to that test's `trace` tag before the matrix counts AC-9 covered, as for AC-1..8. |
| FND-007 | fixed | 7b9515e29: "are admitted in no call in complete-V1 source" now states QSL's admission rather than denying the QSpec grammar route. |
| FND-008 | fixed | 7b9515e29: Inputs now states that a float comes only from a parameter or an IEEE arithmetic result. The new "Conversion into a float" SHALL and AC-8 refuse conversions into a float as `ill_typed`/`type-mismatch`. The probe gives `IllTyped(TypeMismatch)` for `Int[0, 10]`→`Float64`, `Float32`→`Float64` and `Float64`→`Float32`. |
