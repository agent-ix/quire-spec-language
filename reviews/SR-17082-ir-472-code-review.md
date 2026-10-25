---
id: SR-17082
title: "Code review of IR-472 QSL missing enum admission control"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; IR-472; qsl-semantics/tests/it/enum_type_admission.rs, qsl-semantics/tests/it/main.rs; qsl-semantics/src/check/{mod.rs,type_form.rs,check.rs}; FR-091-AC-28, TC-481; RT FR-008, TC-024"
review_set: subset
---
# Code review of IR-472 QSL missing enum admission control

## Summary

Reviewed QSL PR #1503 at 52eef62c88f6dd32b54bf69e728ab5e80fbe6a79. The new test reaches `PackageDeclarations::check` and asserts a typed missing-name refusal, but its package declares no enum, so it does not establish the claimed enum boundary.

## Verdict

**CONDITIONAL** — the generic absent-name path is checked; the enum-specific ownership control needs a fixture containing an admitted enum and a distinct absent enum type form.

## Coverage

- `qsl-semantics/tests/it/enum_type_admission.rs`: examined the complete new test.
- `qsl-semantics/tests/it/main.rs`: examined test module registration.
- `qsl-semantics/src/check/mod.rs`: traced function-signature resolution through `resolve_type_form`.
- `qsl-semantics/src/check/type_form.rs`: traced `named_type` lookup against the scope's declared aliases, composites, enums and object types, including typed `MissingName`.
- `qsl-semantics/src/check/check.rs`: checked that `PackageDeclarations` carries enum bindings.
- FR-091-AC-28 and TC-481: inspected the asserted and tagged behavior.
- RT FR-008 and TC-024: examined the owning-boundary claim and missing-enum control requirement.
- QSL PR #1495: no changed-file overlap with PR #1503.

## Checks

- Diff limited to the new integration test and `tests/it/main.rs`; no production or workflow changes.
- Focused test build was stopped because this worktree had no reusable compiled target and was starting a fresh workspace dependency build. The coder's focused checks are reported in the PR but were not used as review evidence.
- Full aggregate gate remains for the final reviewed head.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The fixture contains only a function with an unresolved bare type name and no admitted enum. It would pass for generic missing type resolution even if the enum-specific admission path were broken, so it does not prove the TC-024 enum ownership control. | qsl-semantics/tests/it/enum_type_admission.rs:14 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | At e6e45e491ee7f83b6b8946fe983e287385d7c35f, `admitted_status()` creates a real admitted `Status` enum binding. The test first checks `Status` positively, then checks `AbsentStatus` in the same admitted enum scope and asserts typed `MissingName` and `MissingDeclaration`. |
