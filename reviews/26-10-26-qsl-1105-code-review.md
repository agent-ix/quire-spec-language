---
id: SR-1634003
title: "Code review — QSL-1105 checked nominal declaration facade"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; qsl-replay/src/application.rs, qsl-replay/src/lib.rs, qsl-replay/tests/checked_nominal_declaration.rs, spec/functional/FR-485-supplied-admission-and-conversion-owner-allocation.md, repository Rust conventions"
review_set: subset
---

## Summary

The direct Rust review checked the checked nominal declaration view against FR-485-AC-13/14, its owner and acquisition boundaries, API construction, and the facade-only integration assertions. No applicable AssuranceProfile was found. No implementation or test defects were found in the reviewed scope.

## Verdict

**PASS** — the added views preserve checked declaration identities, forms, member order and types; refusals remain typed and owner-bound, with coverage through the public facade.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Coverage

The code review compared the changed behavior and tests directly with FR-485-AC-13/14. The new unit tests check ordered record, tuple and union members, retained union identities, owner selection, foreign/wrong-kind/unknown/wrong-acquisition refusals, and inconsistent-form classification. The external integration test uses only the public `qsl_replay` facade plus shared kernel/value APIs and asserts the signature-to-declaration key and ordered members. The test tags bind to FR-485-AC-13/14.

`cargo clippy --locked --workspace --all-targets --all-features -- -D warnings`, formatting, and diff checks passed after the final code adjustment. The full workspace suite passed with five sandbox-incompatible tests skipped before the final behavior-preserving form-conversion/test refactor: `qsl-semantics::model_identity_full_size::ten_thousand_edge_chain` (sandbox denies `sysctl kern.clockrate`) and four `xtask::test_suites` child-process tests (sandbox denies `ps`). The unfiltered run exposed the `sysctl` restriction; the subsequent run excluding it exposed the four `ps` restrictions. After the final refactor, the affected nominal-declaration unit tests and facade integration test passed. The workspace runs otherwise passed, including all `qsl-replay` unit, integration and doc tests.

The installed Quoin exposes no `gap-analysis` command (`quoin gap-analysis --help` reports command not found); the repository-wide matrix attempt did not complete in the available run. Accordingly, this review reports the direct FR-485 scope comparison and targeted trace tags, not a repository-wide matrix result. Plan completion: not assessed.
