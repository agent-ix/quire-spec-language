---
id: SR-2026110216377
title: "QSL-1517 manual gap analysis"
type: SpecReview
scope: "agent-ix/quire-spec-language; qsl-replay/src/spine/lifecycle.rs, qsl-replay/src/spine/lifecycle/tests.rs"
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-480
    type: verifies
---
# QSL-1517 manual gap analysis

## Summary

Ticket: QSL-1517. Quoin matrix did not complete in the available run window, so this is the required manual acceptance-criteria-to-tests check. Actual Quoin gap-analysis remains unavailable.

## Verdict

The existing focused tests cover empty and duplicate source lists, unit/library owner collision, cross-unit name duplicates, sibling import, one profile-set mismatch, and canonical ordering. FR-480-AC-4 still lacks focused assertions for profile identity order mismatch, edition mismatch, and alias variation; header diagnostic location and naming is also not asserted. No test checks refusal source loci for sibling imports.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-60331 | medium | AC-4's same-profile identities in a different order, edition mismatch, and alias-variation cases are not covered by the unit-list tests; the existing header-conflict assertion only checks indices. | qsl-replay/src/spine/lifecycle/tests.rs:171-235 |

## Findings data

The focused tests are `unit_list_preflight_refuses_empty_duplicate_and_library_owner_before_parsing` and `parsed_unit_list_rejects_cross_unit_names_and_orders_distinct_units_canonically`. The latter exercises only different profile sets and checks `HeaderConflict { first: 0, second: 1 }`; there is no assertion for the full AC-4 profile-order, edition, alias, or header-locus cases.
