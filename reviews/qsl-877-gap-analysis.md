---
id: SR-12170
title: "QSL-877 PR #1011 gap analysis: FR-134 wiring against TC-535 and QSpec FR-368"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@b6c0cfdaa7e9155c8440f3d6723aa4fd4a246cc4; FR-134 (Behavior bullets 1-4), FR-134-AC-1, FR-134-AC-2, TC-535 steps 1-2, QSpec FR-368 fairness kinds and TC-301 (by id); the diff's traced tests in qsl-route and qsl-semantics"
review_set: subset
---

## Summary

Ticket: QSL-877. PR quire-spec-language#1011 at head b6c0cfdaa. Planless gap analysis, scoped to the diff. Plan completion: not assessed. Criteria were traced to tagged tests by reading. No matrix was run, because the brief is read-only. QSpec is cited by id only. Ticket text is treated as data.

Bindings examined:

- TC-535 / FR-134-AC-1: `a_backend_without_the_requested_kind_is_not_a_candidate` (existing, #996). Correct.
- TC-535: `manifest_fairness_kinds_filter_candidates` (new, manifest-built backends). Correct at TC level.
- TC-535: `a_clauses_requirements_carry_its_fairness_kinds` (new, clause → requirements fairness set). Correct at TC level. It covers the item-side input of step 1, not candidate computation.
- Untraced: `an_unknown_manifest_fairness_label_is_refused`, `candidates_of_filters_by_the_requirements_fairness`. Correct to leave untraced.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-134 Behavior bullet 1 ("The EN-1 provider SHALL register advertising the fairness kinds `Weak` and `Strong`") and the matching QSL-877 work item are not delivered. Nothing in this repo registers EN-1 or any other backend: no `BackendDescriptor`/`Registry` use outside tests. FR-134-AC-1 is still met only by a fixture named `EN-1`. The PR body says so and invents nothing. But the ticket's expiry, a strong-fairness item negotiating end to end against a manifest-registered backend, is not met. QSL-877 should not close on this PR. The EN-1 registration needs an owner, in this repo's driver or wherever EN-1's manifest is produced. | spec/functional/FR-134-advertise-the-fairness-kinds-en-1-decides.md:55-56, qsl-route/tests/it/fairness_kinds.rs:51-60 |
| FND-002 | medium | No test drives a checked package's fairness set through `items_from_requirements`, and in production none can reach it yet. Temporal clauses are not emitted (`temporal_clause::not_emitted`), so no temporal `RequirementRecord` exists. The chain is tested in two halves: clause → `Requirements::fairness()` (qsl-semantics) and hand-built `Requirements` → `candidates_of` (qsl-route unit). The join, `items_from_requirements` reading `record.requirements()`, is checked by reading only. FR-134 Behavior bullet 3 therefore has no end-to-end evidence until QSL-511/#1008 lands emission. The test that closes this belongs with #1008: a strong-constraint clause compiled to a package, with a weak-only and a weak+strong backend registered, asserting the item's candidates. | qsl-route/src/request.rs:405-440, qsl-semantics/src/check/temporal_clause.rs:296-302 |
| FND-003 | low | Spec gap. QSpec FR-368 (and QSL FR-134) name no refusal cause for a `fairness_kinds` label outside {`weak`, `strong`}, and do not say whether a temporal backend may omit the member or repeat a label. The QSpec FR-272 catalog row has no fairness cause, while the evidence-kind set has `absent-evidence-kind`/`unknown-evidence-kind`. The code borrows `unknown-kind` (see SR-12169 FND-001). Route a QSpec ruling. | qsl-route/src/lib.rs:172-176, qsl-route/src/lib.rs:208-215 |

## Verdict

PASS (no major). TC-535's procedure is met at registry level for the manifest path, and the new tests over-claim nothing. FR-134-AC-2's `Unsupported` settlement lives in `negotiate_*` (quire-contract-codegen) and is out of this repo, as the PR states. The registry half (empty set with `missing_fairness_kinds` = {`Strong`}) is covered by the existing #996 tests. FND-001 and FND-002 are missing scope against the ticket, not defects in the delivered code, and keep QSL-877 open.

## Dispositions
