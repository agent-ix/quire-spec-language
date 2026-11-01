---
id: SR-1988009
title: "Manual gap analysis — QSL-1398 admitted identity row projections"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; QSL-1398 acceptance evidence; qsl-package/src/emit/tests/selected_model_rows.rs, qsl-package/src/emit/tests/timed_source.rs, qsl-package/src/emit/tests/variant_types.rs, qsl-package/src/emit/tests/state_transition.rs; producer context qsl-semantics/src/model/intake/unit.rs and qsl-package/src/emit.rs"
review_set: subset
---

## Summary

Ticket: QSL-1398. Manually compared the ticket's acceptance evidence with the QSL producer and package tests. Matching tuple and source-unit rows, omitted tuple/source-unit witness refusal, enum witness/refusal coverage, and distinct model-owned operation anchors have corresponding test coverage. The full Quoin gap-analysis command is unavailable in the installed Quoin 0.29.1 CLI (quoin gap-analysis --help returned command gap-analysis not found). The repository-wide quoin matrix --repo . --json emitted no output during several minutes and was interrupted (exit 130); no whole-repository matrix or reverse-gap result is claimed.

## Verdict

CONDITIONAL — the scoped manual acceptance-to-test check found no QSL-1398 gap. A full repository-wide gap-analysis could not be completed with the installed CLI.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No scoped gaps found (placeholder) | - |

## Coverage

- Tuple: selected_tuple_rows_match_emitted_model_nodes_and_skip_absent_nodes checks producer node correspondence and absence; selected_tuple_row_projects_the_admitted_type_and_missing_row_refuses checks the selected original identity, projected NodeKey, and typed refusal when the row is absent.
- Source-owned unit: a_checked_time_source_emits_and_reads_back_its_resolved_nodes checks the emitted structural node, qualified declaration, source authority/identity, declared UnitId, successful IR projection, and typed refusal when the unit row is absent.
- Enum: selected_model_enum_rows_join_and_refuse_missing_mismatched_duplicate_and_foreign_rows checks producer rows and the enum admission failures.
- Model-owned operation: state-transition coverage verifies two distinct full anchor IDs for one operation while retaining its structured selected operation identity.
- Plan completion: not assessed.
- Optional semantic review: skipped; not requested.
- Quoin gap-analysis command unavailable; full repository matrix attempt stopped without output as recorded above. This is a scoped manual check, not a repository-wide PASS claim.
