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

## New findings (disposition pass 1)

Reviewed at 8a09ad0b6 against the merged STD-193 text (FR-134-AC-3..AC-6, FR-057 handoff, TC-535 V1..V17 and steps 4-5).

TC-535 vector coverage (`qsl-route/tests/it/fairness_kinds.rs`):

- V1-V3: `v1_to_v3_admitted_lists_advertise_their_kinds` (AC-5).
- V4, V5, V6, V11, V14: `v4_v5_v6_v11_v14_...` (AC-3).
- V12, V13 (plus `{}`): `v12_v13_...` (AC-3).
- V7, V8, V10, V15: `v7_v8_v10_v15_...` (AC-4).
- V9: `v9_...` (AC-5).
- V16, V17: `v16_v17_...` (AC-6).
- Step 4 (V4, V7, V9 under a held identity): `a_refusal_leaves_the_earlier_registration_standing` (AC-5; see SR-12169 FND-003).
- Step 5: `a_non_temporal_backend_ignores_the_fairness_member` (AC-5).

All 17 vectors are covered. Every AC-3/4/5/6 tag sits on a test that can fail on that AC, except the one assertion named in SR-12169 FND-003. `a_compiled_clauses_fairness_set_filters_its_items_candidates` and `manifest_fairness_kinds_filter_candidates` carry TC-535 only. Out of scope, as before: TC-535 step 2's `Unsupported` settlement (`negotiate_*`, quire-contract-codegen) and FR-134 Behavior bullet 1.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-004 | low | ADR-013 C-28 still lists what the driver passes to QSL as identity, (kind, mode) labels, `domains` and `evidence_kinds`, with refusal causes up to `unknown-evidence-kind`. It omits the `fairness_kinds` member and the three fairness causes. FR-057 (driver handoff) and FR-134 now state both, so the C-28 row is out of step with the requirement it governs. Add `fairness_kinds` (as supplied, for a `temporal-satisfaction` backend) and the three QSL-proposed causes to the row. | spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md:1025, spec/functional/FR-057-admit-shared-capability-kinds.md:248-253 |

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | deferred | No backend registration site exists in this repo (no `BackendDescriptor`/`Registry` use outside tests at 8a09ad0b6), so FR-134 Behavior bullet 1 cannot be delivered here. The PR body states this and invents nothing. The leader routes an owning ticket for EN-1's registration (driver/provider side). Until then QSL-877's EN-1 work item stays open. |
| FND-002 | fixed | 150d8368d. `tests/it/fairness_item_path.rs` compiles a ConfigVersion temporal clause, runs `items_from_requirements` against backends admitted through `descriptor_from_manifest`, and asserts `strong` → [`both`], `weak` → [`both`, `weak-only`], and no fairness → both. It is reachable because #1008 builds temporal records through `TemporalClaim::requirements`. |
| FND-003 | fixed | 2b5ddc17e, on the merged STD-193 spec (#1012). FR-134 "Manifest `fairness_kinds` admission" names `unknown-fairness-kind`, `duplicate-fairness-kind` and `absent-fairness-kinds`, the absent-member and repeat rules, and non-temporal ignore. The code implements exactly that. The remaining QSpec catalog ruling is tracked as STD-194 in FR-134 Remaining work. |
