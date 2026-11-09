---
id: SR-20261108210000000003
title: "Trace and gap review of authored-clause request routing"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; QSL-1819; PR 2603; changed qsl-analyze request/provider path and its FR-280/FR-281 obligations"
review_set: subset
relationships:
  - {target: "ix://agent-ix/quire-spec-language/FR-281", type: references}
  - {target: "ix://agent-ix/quire-spec-language/FR-280", type: references}
---

## Summary

PR-scoped gap review, explicitly narrower than a repository-wide assurance audit. Changed public request/routing behavior has an existing owner in FR-281 and FR-280; no source or test stub was found in this diff. No plan was supplied.

## Verdict

PASS for the changed slice, with the computed repository-wide matrix unavailable at this review checkpoint. This is not a repository-wide gap-analysis PASS or a claim that all FR-281 criteria or the matrix cells are satisfied.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Coverage

Plan completion: not assessed

Reconciliation: manual inspection of declared trace attributes and governing requirements for this diff. `quire matrix --scope /home/peter/dev/worktrees/QSL1819b-route-clause --format json` was launched; it was still actively computing with no result at this checkpoint. No computed coverage counts or repository-wide completeness claim is made. The healthy command was not cancelled solely because it was slow. No run-evidence store was read.

Examined units: FR-281-AC-2 (refuted and replay), FR-281-AC-11 (the EN-1 unbounded-root refusal clause), FR-280-AC-2 (provider/direct operation parity), and the added FR-281 negotiated-clause-member paragraph. Context-only: FR-281-AC-6 (EN-5-specific budget case, deliberately not claimed by the supplementary EN-1 budget test) and FR-280-AC-6 (existing ordering/admission behavior).

Bindings inspected and correct:

- clause_route::analyze_routes_an_authored_clause_to_en1_and_settles_refuted_with_replay -> FR-281-AC-2, partial route evidence. Existing clause_item::a_clause_a_transition_breaks_settles_refuted_with_a_replayed_counterexample exercises actual counterexample possession and reproduction.
- clause_route::an_unbounded_extent_clause_settles_unsupported_through_the_provider_entry -> FR-281-AC-11, EN-1 concrete refusal clause. Existing analyze::tests::engine_refusals_settle_by_the_refusal_table supplies the record-level payload assertions.

The five public behavior additions inventoried are clause request data, routed clause membership, heterogeneous routed-work conversion, generic analyze dispatch and generic provider dispatch/fallback. All map to FR-281 negotiated items and FR-280 provider semantics; zero unowned additions, source stubs or hollow tests were found in the changed slice. The new tests call real compilation, checked-package admission, EN-1 work and production replay, and assert concrete results/diagnostics rather than only success or existence.

Optional repository-wide semantic audit: skipped; the PR's test-oracle review was performed as part of code review. Matrix integration remains in the separate QSL-12 lane, and EN-5 provider request routing remains in QSL-1820. Neither is marked complete here.
