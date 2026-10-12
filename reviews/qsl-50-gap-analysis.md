---
id: SR-7837
title: "QSL-50 gap analysis of PR #782 operation normalization"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@2c5d8bf910658162d2568f5a4cd8df52a19f9404; PR #782 base 8d293ebf3b90878eda2703e717bd519273ea9b70; QSL-50 operation-member qualify, inherit, and redefine derivation/accounting; qsl-semantics/src/model/normalize.rs; qsl-semantics/tests/it/model_normalization.rs; ticket acceptance and related TC-196 / FR-151-AC-2 test binding"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-150
    type: references
---
## Summary

Manual acceptance-criteria-to-test and oracle-strength review of QSL-50 against the exact PR diff. The ticket's requested operation-member facts, work accounting, and operation redefinition behavior have direct assertions in `operation_redefinition_is_charged_like_a_field_redefinition`, `operation_redefinition_group_with_two_or_more_redefiners_is_charged_a_conflict_check`, `operation_diamond_derivation_conflict_reports_the_full_typed_payload`, and `operation_redefinition_group_resolved_by_a_dominating_owner_completes`.

## Verdict

**CONDITIONAL** — no implementation or test-oracle gap was found in the requested ticket behavior; full Quoin matrix output could not be obtained in this environment, so the repository-wide traceability axis remains unverified here.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No ticket-scope acceptance gap found (placeholder required by the SpecReview table schema). | - |

## Coverage

- Ticket behavior examined: operation members receive qualify facts in phase 2, inherit facts in phase 3, and paired redefine facts for each reachable operation redefinition in phase 4; limits and work totals account for those facts.
- Test binding examined: the four tests named above are tagged `QSpec-TC-196` and `QSpec-FR-151-AC-2`. The latter binding is relevant to operation redefinition resolution; the ticket's precise fact derivation and accounting behavior is independently checked by assertions in those tests.
- Oracle strength: tests inspect the actual `EffectiveView` derivation rules and ordered inputs, visibility of inherited/redefining/winning members, refusal payloads, exact `NormalizeFact` and conflict-check charges, and exact work-limit stopping points. These can fail if operation facts are omitted, misordered, misbound, or mischarged.
- Quoin matrix attempt: `quoin matrix --repo /tmp/qsl50-review-2c5d8bf --json` produced module diagnostics without a usable matrix; the direct `quire matrix --scope /tmp/qsl50-review-2c5d8bf --format json` attempt likewise did not yield a report. Static repository-wide coverage is therefore unverified. This artifact records the required manual ticket-scope check separately.
- Plan completion: not assessed.
- Gate note: the reported `make ci` result is exit 2 on the named bench test, with the same isolated failure reported on the clean base. It remains a failing aggregate gate, not a green result.
