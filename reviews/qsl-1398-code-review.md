---
id: SR-1988008
title: "Code and Rust review — QSL-1398 admitted tuple and source-unit joins"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; Cargo.toml, Cargo.lock, qsl-package/src/emit/tests/selected_model_rows.rs, qsl-package/src/emit/tests/timed_source.rs; context qsl-semantics/src/model/intake/unit.rs, qsl-package/src/emit.rs, qsl-package/src/emit/tests/variant_types.rs, qsl-package/src/emit/tests/state_transition.rs"
review_set: subset
---

## Summary

Ticket: QSL-1398. Reviewed PR #2003 at 45d29404e64084f6c6ce747c2e3f21c7c99bc900 against base 6664f6818f42ad87785674cfdd25b481a73fbc0f. The diff pins quire-contract-model to IR #774 and adds package-level tuple and source-unit projection/missing-witness tests. The review also checked the existing enum and operation producer tests and the producer paths already merged in #1989. The Rust review found no correctness, style, lifecycle, or boundary defect in the diff. No applicable AssuranceProfile was found.

The code review used coder-reported focused test, no-default package check, Clippy, formatting, and diff-check evidence; no new compilation was run by this reviewer.

## Verdict

PASS — no code-review findings. The tuple/source-unit rows in the emitted package are joined to the admitted package through IR's public projection API, and omission returns the typed unavailable-witness result without exposing a partial projection. The pin is commit-specific and the lockfile matches it.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Examined obligations

| Unit | Role | Excerpt |
| --- | --- | --- |
| QSL-1398 acceptance evidence | examined | Producer rows match the admitted selected package/model for tuple, source-owned unit, enum declaration/member, and model-owned operation anchors. |
| QSL-1398 acceptance evidence | examined | Multiple admitted anchors for one operation retain distinct anchor IDs while sharing the selected operation identity where appropriate. |
| FR-230-AC-32 | examined | A checked timed model reaches S4 and I2 read-back without TimedModelNotEmitted; the timed-model root's model reference equals the unique checked selected-package semantic node resolved for its source model alias, its source unit reference equals the resolved unit node, and the emitted timed-model nodes reconstruct the same time source, clock places, guards, resets, invariants, urgency and normalized constraints. |
| FR-230-AC-38 | examined | For time M dense unit u and time M tick O period p unit u, S3 resolves model alias M against the selected package and retains its alias and unique checked semantic-node NodeKey on the checked time source. A package with no semantic node denoted by M refuses missing_declaration/missing-name at M; a package with multiple candidate semantic model nodes refuses ambiguous_declaration/ambiguous-name at M, regardless of declaration or graph order. |
| selected_tuple_row_projects_the_admitted_type_and_missing_row_refuses | examined | The admitted selected tuple projects with its original identity and NodeKey; omitting its row returns UnavailableIdentityWitness. |
| a_checked_time_source_emits_and_reads_back_its_resolved_nodes | examined | The selected source-unit row carries the qualified unit name, source authority/identity and UnitId; the admitted unit projection succeeds and omission returns UnavailableIdentityWitness. |
| selected_model_enum_rows_join_and_refuse_missing_mismatched_duplicate_and_foreign_rows | examined | Existing enum tests exercise the emitted declaration/member witnesses and missing, mismatched, duplicate and foreign-row refusals. |
| state_transition.rs distinct model-owned anchors | examined | Existing operation tests assert distinct full anchor IDs and a shared structured selected operation identity where the source operation is the same. |

## Coverage and limits

Bindings checked: a_checked_time_source_emits_and_reads_back_its_resolved_nodes is correctly tagged FR-230-AC-32 and FR-230-AC-38. The new QSL-1398 tuple/source-unit cases are manually traced to the ticket acceptance evidence; the changed behavior has no corresponding QSL-local FR in this PR's scope. The optional semantic review was not requested. No repository-wide matrix result is claimed here.
