---
id: SR-12302
title: "Gap analysis of quire-spec-language PR #1180 against QSL-950, FR-091-AC-44 and TC-483"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@c4f570dd90fa9fc0a390463dd768347eeb2f0c80; PR #1180 diff against origin/main; spec/functional/FR-091-produce-value-forms-and-assemble-package-declarations.md (dimension and unit section, AC-44); spec/test-cases/TC-483-assembler-admits-dimensions-and-units.md (step 5); qsl-semantics/src/check/assemble.rs; qsl-replay/tests/it/declaration_name_collisions.rs"
review_set: subset
---
# Gap analysis of quire-spec-language PR #1180

## Summary

Ticket: QSL-950. The ticket text is treated as data, and each claim was
re-measured. Plan completion: not assessed (planless).

The ticket's defect is that QSL accepts a source that the strict reader
then refuses with `AmbiguousDeclaration`/`AmbiguousName`, because the reader
keys every declaration name in one namespace. The ticket offers two
resolutions, and this PR takes the QSL-refusal one.

Examined:

- FR-091-AC-44, the new collision clause. A unit beside a record or a
  dimension of one name refuses at E3 as `AmbiguousTypeName`
  (`ambiguous_declaration`/`ambiguous-name`), with the name and both spans,
  unused, in either order. Covered by
  `unit_record_and_unit_dimension_collisions_are_typed_before_emission`
  (8 variants) and by
  `unit_record_collisions_and_duplicate_units_are_typed_refusals`. Both
  pass, and the first fails when the check is removed.
- FR-091-AC-44, the distinct-name clause. A unit and record with distinct
  names compile through `compile_admitted` and are admitted by strict V2.
  Covered by `distinct_and_case_distinct_names_compile_and_pass_strict_ir_admission`,
  which passes.
- FR-091-AC-44, the two-unit and `record Quantity` clauses. Existing
  tests, unchanged, pass.
- TC-483 step 5 and its expected results match the tests above.
- Refusal before emission: the refusal is `CompileRefusal::Assembly`, so no
  package is returned and the strict reader never sees one.
- Catalog code: `ambiguous_declaration`/`ambiguous-name` is listed in QSpec
  FR-272 "typed diagnostic cause".
- Imports and packages: a domain-package declaration is reached only as
  `M::T` under its alias, so it cannot share a source unit's bare name. No
  cross-package pair is in scope.

Bindings: both replay tests are tagged `TC-483`/`FR-091-AC-44` and test
what they claim. The assembler test is tagged the same and is correct.

## Verdict

NOT MERGEABLE. FND-001 is a major. The ticket's measured defect class is
only partly closed: seven sibling declaration pairs still compile and
are refused by the strict reader.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | QSL-950's acceptance is that QSL does not accept a source the strict reader refuses for a shared declaration name. That still fails, measured at c4f570dd9 with each declaration used: unit/tuple, unit/union, unit/enum, unit/function, dimension/record, dimension/enum and dimension/function all compile through `compile_admitted`, and `CheckedPackageV2::read` then refuses each one with `AmbiguousDeclaration`/`AmbiguousName` at `declaration/qualified_name`. The PR's tests cover only the record and dimension pairs with a unit, so this gap is invisible to them. Fix it in this PR: refuse a unit or a dimension that shares a name with any emitted declaration, and add a used-declaration test per pair. | qsl-semantics/src/check/assemble.rs:733-762; qsl-replay/tests/it/declaration_name_collisions.rs:29-77 |
