---
id: SR-12303
title: "Spec review of quire-spec-language PR #1180: FR-091 unit-name collision rule and TC-483 step 5 (QSL-950)"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language@c4f570dd90fa9fc0a390463dd768347eeb2f0c80; spec/functional/FR-091-produce-value-forms-and-assemble-package-declarations.md (dimension and unit section, AC-44); spec/test-cases/TC-483-assembler-admits-dimensions-and-units.md (step 5 and expected results); context: QSpec spec/functional/type-model/FR-142-evaluate-quantities-and-units.md"
review_set: subset
---
# Spec review of quire-spec-language PR #1180

## Summary

Ticket: QSL-950. This review covers the new FR-091 paragraph, the
rewritten FR-091-AC-44 and TC-483 step 5. It checks them for consistency,
for testability, and against the authoritative QSpec FR-142.

`make check-spec-validation` passes at c4f570dd9, with no new warning on
the changed files. The new SHALL statement is EARS-conformant, and it is
testable and tested.

## Verdict

Not mergeable as written, because of FND-001. The new rule contradicts
QSpec FR-142 "Namespaces" and FR-142-AC-13, which say that a unit and a
record of one name coexist. The PR drops the AC's citation of
FR-142-AC-13 instead of recording the conflict. There is no QSpec ticket
for the change. Either:

- file the QSpec amendment and cite it as `Remaining work: <id>`, or
- take the ticket's other resolution: the IR reader keys units and
  dimensions in their own namespace.

FND-002 is the spec side of SR-12302 FND-001. The rule names a subset of
the colliding declarations. FND-003 and FND-004 are lows.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-091 now says the checker SHALL refuse a unit that shares a name with a record. AC-44 says the same. QSpec FR-142 "Namespaces" says dimensions, units and the type namespace (records, tuples, unions, enums, aliases, functions and so on) are three namespaces, and FR-142-AC-13 says "A unit and a record of one name coexist". The QSL spec now contradicts the language spec it implements. The PR deletes AC-44's `(QSpec FR-142-AC-13)` citation rather than recording the divergence, and no QSpec ticket amends FR-142 to match the strict reader's single-namespace rule. | spec/functional/FR-091-produce-value-forms-and-assemble-package-declarations.md:481-488, 972 |
| FND-002 | medium | The paragraph's own premise is that emitted declaration names cannot collide. The rule it then states covers only "a record or dimension". The measured reader collisions also include tuple, union, enum and function with a unit, and record, enum and function with a dimension. The rule should name every emitted declaration, or state the general invariant. Otherwise the code (SR-12302 FND-001) follows a spec that is too narrow. | spec/functional/FR-091-produce-value-forms-and-assemble-package-declarations.md:481-488 |
| FND-003 | low | AC-44's Verification cell changed from `Test (TC-483)` to `Test`. Every sibling row names its TC, for example `Test (TC-1052)`, and TC-483 still verifies the AC. | spec/functional/FR-091-produce-value-forms-and-assemble-package-declarations.md:972 |
| FND-004 | low | AC-44 now bundles nine checkable clauses in one criterion: parse, resolve, two missing-name refusals, type-mismatch, the collision, the distinct-name strict admission, duplicate units and `Quantity` syntax. The collision and strict-admission clauses this PR adds could be their own AC, so a failure points at one clause. | spec/functional/FR-091-produce-value-forms-and-assemble-package-declarations.md:972 |
