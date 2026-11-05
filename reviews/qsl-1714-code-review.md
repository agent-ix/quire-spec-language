---
id: SR-20261105001151
title: "Code review — quire-spec-language QSL-1714/QSL-1715"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; qsl-analyze/src/claim_family.rs, qsl-analyze/tests/it/claim_family.rs, qsl-analyze/tests/it/operation_contract.rs, qsl-package/src/emit/tests/one_occurrence.rs, qsl-package/src/emit/tests/step_relation.rs, qsl-replay/src/contract_check.rs, qsl-semantics/src/check/claims/map.rs, qsl-semantics/src/check/lowering.rs, qsl-semantics/src/check/mod.rs, qsl-semantics/src/check/node_key/mod.rs; FR-507-AC-7, FR-507-AC-11, QSpec FR-290, FR-477, FR-485"
review_set: subset
---

## Summary

Reviewed PR #2354 at frozen head `895312967058b6bbb4963eb00fabd581cb26bb68` against the QSL-1714/QSL-1715 carrier requirements, Rust implementation and tests. The carrier changes and the focused replay/admission assertions align with the intended shared-occurrence behavior; two added tests lack criterion trace tags required by this repository's Rust testing convention.

## Verdict

**CONDITIONAL** — two low-severity traceability findings remain on new tests.

## Scope

- Examined FR-507-AC-7 and FR-507-AC-11, with QSpec FR-290, FR-477 and FR-485 as context for the occurrence and role contracts.
- Examined all ten changed Rust files listed in `scope`, including the map implementation, Boolean expression lowering, family reader, replay selection, and the focused tests.
- No applicable AssuranceProfile applies to these changes.
- Existing focused test and formatting results were supplied by the lead; no aggregate workspace check, full suite, or dependency audit was run in this review pass.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The new map unit test exercises the shared occurrence/capability behavior covered by FR-507-AC-7 but has no `#[trace]` binding, so this regression assertion cannot be attributed by the repository's traceability tooling. | qsl-semantics/src/check/claims/map.rs:314 |
| FND-002 | low | The new Boolean-clause reader test asserts FR-507-AC-11's expression-node claim/declaration mapping but has no `#[trace]` binding. | qsl-analyze/tests/it/claim_family.rs:519 |

## Review Notes

Rust review found no concrete implementation defect in the changed carrier paths. `RequirementMap` now indexes multiple capabilities per occurrence and rejects ambiguous unqualified lookup; the Boolean reader checks the Boolean expression type; the bound step-relation code claim is selected by `OperationContract`; and the tests exercise Contract IR admission, family selection and replay-arm selection. The added expression/body lowering retains the existing body binding and adds the claim/declaration origins at the expression node.

The new `one_occurrence_can_hold_records_of_distinct_kinds` test and `a_boolean_value_clause_is_read_on_its_expression_node` test should each carry the criterion id they assert (respectively FR-507-AC-7 and FR-507-AC-11). Other new integration tests in the diff carry trace tags.

## Gate Evidence

- `cargo fmt --all -- --check`: pass, as reported in the PR verification body.
- Focused tests: pass, as reported in the PR verification body.
- Combined workspace check, Clippy configurations and full suite: deferred to the lead's aggregate gate per review dispatch.
- `cargo deny`: not run; this diff changes no dependency manifests or lockfiles.
