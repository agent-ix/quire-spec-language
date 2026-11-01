---
id: SR-202610310148
title: "Gap analysis — QSL-1440 scalar validity proof"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; spec/; qsl-analyze/src/value_check/mod.rs, qsl-analyze/tests/it/value_validity.rs, qsl-replay/src/value_check.rs"
review_set: subset
---

## Summary

Ran a focused acceptance-to-test and code check for QSL-1440's scalar-operation proof against QSpec `FR-484-AC-6` and vector `VV-18` from QSpec commit `87848ee9`. The repository-wide matrix and reverse-gap audit could not complete because both `quoin matrix --repo . --json` and `quire matrix --scope . --format json` stalled without producing results; a manual check covers this change's traced criterion only.

## Verdict

**CONDITIONAL** — the focused criterion is traced to a behavioral integration test and matches the implementation; repository-wide matrix and reverse-gap coverage remain unavailable.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No gaps found (placeholder). | - |

## Coverage

- Reconciliation: manual acceptance-criteria-to-test check; Quoin and Quire matrix commands stalled before producing output.
- Plan completion: not assessed
- Criteria: QSpec FR-484-AC-6 inspected; `qsl-analyze/tests/it/value_validity.rs::the_scalar_operation_vectors_settle_as_the_vectors_state` carries `QSpec-FR-484-AC-6` and asserts VV-18's proved/exhaustive result.
- Semantic review: ran over the changed FR-484-AC-6 requirement/test/code triple. The test constructs a real QSL package from the vector's `a * 1` operand, with `a: Int[0,3]` and result `Int[0,5]`, then checks the terminal result and exhaustive basis. The source returns `Holds` only after the entire function completes and the selected occurrence lies below eager arithmetic/coercion nodes; conditional and short-circuit paths stay nondecisive. Refutation and other-stop behavior remain unchanged.
- Reverse-gap inventory and repository-wide stub counts: not assessed because the matrix tooling did not return and this review was scoped to the changed behavior.


## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-002 | medium | The provider's all-unreached result is described as vacuous even when a prior undefined expression masks a site whose path condition is satisfiable. | FR-062, qsl-analyze/src/value_check/mod.rs:13-15 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-002 | still-open | `NotReached` also results when evaluation stops at an earlier undefined expression. In that case the path condition is not vacuous, but the provider has not established the selected operation's result. |
| FND-002 | fixed | The wording now distinguishes vacuous paths from earlier undefined expressions that mask the site. |
