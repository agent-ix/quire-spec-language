---
id: SR-20261104173646
title: 'Gap analysis of PR #2306: QSL-1496 TC-149 checked-route trigger identity'
type: SpecReview
analysis: gap-analysis
scope: 'agent-ix/quire-spec-language@3ca35d2f5 trial-merged with origin/main 60ab2e913; PR #2306 diff: FR-053-AC-7, FR-053-AC-8, FR-053-AC-9, FR-052-AC-12, TC-149; ticket QSL-1496'
review_set: subset
date: '2026-11-04'
---

## Summary

Ticket: QSL-1496. PR: quire-spec-language#2306. Planless gap analysis scoped to the diff, checked against TC-149 and FR-053 as they stand on origin/main 60ab2e913. That main commit (#2307) re-homed TC-149 to the evidence route after the PR was written and added expected results to rows 4 and 5. The PR body's four "deviations" are resolved by #2307; the rows it added are not yet tested. Plan completion: not assessed. `quire trace --id` timed out after 120 s, so tag coverage was checked by grep over the merged tree.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-053-AC-7 distinct identities stay distinct; repeat delivery one instance | spec/functional/FR-053-preserve-opaque-semantic-trigger-identity.md | examined |
| FR-053-AC-8 one byte changes identity; no field stands in | spec/functional/FR-053-preserve-opaque-semantic-trigger-identity.md | examined |
| FR-053-AC-9 empty/absent/stale identity refused, no aliased instance | spec/functional/FR-053-preserve-opaque-semantic-trigger-identity.md | examined |
| FR-052-AC-12 correction names direct predecessor; bad predecessors refuse | spec/functional/FR-052-*.md:64 | examined |
| TC-149 rows 1-6 | spec/test-cases/TC-149-checked-package-opaque-trigger-identity.md | examined |
| Pre-existing TC-149/FR-053 tag site | tests/it/compiled_protocol_v2.rs:1552-1566 | context_only |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | TC-149 row 4 (main) is partly untested. Missing: a redelivery of `IDA` with a different anchor and one with `guard: Some(false)`, each giving one instance with the first capture kept; an exhaustive `let TriggerDelivery { identity, receipt, anchor, payload, guard, captures } = ...` so a new delivery member breaks the build (the AC-8 timestamp/display/declaration-position clause); and the distinct-identity deliveries listed in both orders giving the same set of instances. Fix: add these three cases to `no_other_field_substitutes_for_the_identity` or a sibling test tagged FR-053-AC-8 and TC-149. | qsl-eval/tests/it/checked_route_trigger.rs:176-247 |
| FND-002 | medium | Refusals are checked by `cause` only. TC-149 row 4 requires the contradiction record to carry `contradicted_identity` equal to the identity bytes; row 5 requires `surplus_capture_identity`/`surplus_capture_index` and an empty charge log for every refusal (`trigger-required`, `trigger-unknown`, `surplus-capture`). The route emits these fields (qsl-eval/src/temporal/evidence/call.rs:1136-1152) but no checked-route test reads them, so a refusal naming the wrong identity passes. Fix: match the `CallAdmissionFailure::Refused` record and assert those fields, and run each row-5 refusal with a local `Meter` asserting `admitted_charges().is_empty()`. | qsl-eval/tests/it/checked_route_trigger.rs:204-217,278-287 |
| FND-003 | low | FR-052-AC-12's positive half ("a valid correction names its exact direct predecessor ... and the predecessor result remains unchanged") is asserted only in `a_chain_of_corrections_names_each_direct_predecessor_and_leaves_earlier_results`, which the PR does not tag FR-052-AC-12. The three tagged tests cover only the refusal half. Fix: add FR-052-AC-12 to that test's trace list. | qsl-eval/tests/it/temporal_relation.rs:74-84 |

## Verdict

Conditional. The six tests assert real behaviour for FR-053-AC-7 to AC-9 and fail under the PR's stated mutations, but FND-001 and FND-002 leave TC-149 rows 4 and 5 (as now specified on main) partly uncovered. Fix both in this PR; FND-003 is a one-line tag.

- **Row coverage that is complete:** rows 1, 2, 3 and 6 match TC-149 on main, including the ledger-carried repeat delivery and the retained `IDB` instance. Row 5's constructor, `trigger-required`, `trigger-unknown`, no-delivery `Inactive` and surplus cause are covered.
- **The `v2:trg-A` case** stays a distinct second instance, which covers the "no translation" clause of AC-9.
- **No vacuous tag:** every new `#[trace]` sits on a test that asserts the tagged AC's behaviour.
- **No ceremony, no weakened gate, no QSpec content.**

## Dispositions

- FND-001: fixed. Anchor and guard-false redeliveries keep one instance and the first capture; an exhaustive `TriggerDelivery` destructure test and both-order distinct-identity deliveries are added, tagged TC-149 and FR-053-AC-8.
- FND-002: fixed. `refusal_fields` checks `contradicted_identity`, `surplus_capture_identity`, `surplus_capture_index` and an empty charge log for each refusal.
- FND-003: fixed. FR-052-AC-12 is tagged on the chain-of-corrections test.
