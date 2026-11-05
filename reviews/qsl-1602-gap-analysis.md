---
id: SR-20261105101006
title: "Gap analysis of QSL-1602 PR #2371"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; FR-091-AC-50, FR-091-AC-55, FR-091-AC-72; changed Rust source and tests in qsl-semantics"
review_set: subset
---

## Summary

The quoin gap-analysis command is unavailable in the installed CLI (`quoin gap-analysis` returns “command not found”), and a full computed `quire matrix` run did not finish within 90 seconds, so the matrix result is not claimed. Manual inspection found tagged coverage for the new package-unit cases, but the `FR-091-AC-72` conversion truth claim lacks runtime evidence. Plan completion: not assessed.

## Verdict

CONDITIONAL. Manual acceptance-to-test review found one coverage gap in the new AC-72 path. No full-repository matrix, reverse-gap, or stub audit is claimed because the required matrix computation did not complete.

## Scope

- `FR-091-AC-50` (examined): `the assembler admits every form with no unresolved-name and no duplicate error: the source Time and R::Time are two dimensions and the source s and R::s two units`
- `FR-091-AC-55` (examined): `FR-142-AC-19's unit tick : M::Rate = 1/2 * M::us, typed by the package dimension Rate, is admitted`
- `FR-091-AC-72` (examined): `function k1 using v(d: Quantity<tick>): Boolean pure { convert<Quantity<M::us>>(d) = 1 M::us } checks and is true for d = 2 tick`
- `qsl-semantics/tests/it/dimension_equivalence.rs` (examined): `fn a_function_converting_a_package_typed_unit_to_a_package_unit_checks() {`
- `qsl-semantics/src/check/assemble/units.rs` (examined): `Package(DeclarationKey),`

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | `a_function_converting_a_package_typed_unit_to_a_package_unit_checks` calls `.check()` but never evaluates `k1` with `2 tick`; therefore the test does not verify AC-72's required `true` result. | FR-091-AC-72; qsl-semantics/tests/it/dimension_equivalence.rs:1029-1059 | correct-requirement-no-evidence |
