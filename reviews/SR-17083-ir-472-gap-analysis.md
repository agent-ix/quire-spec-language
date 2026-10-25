---
id: SR-17083
title: "Gap analysis of IR-472 QSL missing enum admission control"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; IR-472; qsl-semantics/tests/it/enum_type_admission.rs; FR-091-AC-28, TC-481; RT TC-024"
review_set: subset
---
# Gap analysis of IR-472 QSL missing enum admission control

## Summary

Reviewed the new test's requirement trace and the production lookup path. The test proves a generic absent bare type name is refused, but its `FR-091-AC-28` tag claims coverage of a different enum behavior.

## Verdict

**CONDITIONAL** — correct the test-to-criterion binding and make the missing-enum control sensitive to an admitted enum scope.

## Coverage

Plan completion: not assessed.

- FR-091-AC-28 requires an admitted `Status` enum, resolved `ValueType::Enum`, successful calls, a `Color` ordering refusal and a missing `Status::GONE` member refusal. The new test exercises none of those conditions.
- TC-481 mirrors those steps. Its separate step 6 tests unresolved `Shade` at the assembler boundary, whereas the new test calls `PackageDeclarations::check` on a constructed declaration.
- RT TC-024 calls for a direct missing-enum type-form control at the QSL checker boundary; the new test uses that boundary but has no admitted enum in its fixture.
- `qsl-semantics/src/check/type_form.rs` `named_type` handles all missing names with the same typed `MissingName` branch. No unowned production code or stub was introduced.
- Targeted `quire matrix` was stopped because it began a broad repository scan; direct tag and source comparison supplied the relevant evidence. Optional full semantic review was not invoked.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The test tags FR-091-AC-28, but it asserts only rejection of an absent type name. That AC's missing-declaration case is an absent `Status::GONE` enum member in a package with an admitted `Status` enum; the binding overstates coverage. | qsl-semantics/tests/it/enum_type_admission.rs:11 |
