---
id: SR-1751002
title: "PR #1751 (QSL-544) gap analysis: FR-144-AC-1, AC-5 (TC-549)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; git diff origin/qsl-1181e-memory...ef62a25cc; qsl-analyze/tests/it/refinement_request.rs, deadlock_item.rs, time_lock_item.rs; tests/support/refinement_cas.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-144
    type: reviews
---

# PR #1751 gap analysis

## Summary

Ticket: QSL-544. Plan completion: not assessed.

**ACs examined:**
- **FR-144-AC-1**, by `a_refinement_writes_one_temporal_record_routed_to_the_explicit_state_provider` and `a_refinement_brings_only_its_concrete_subjects_deadlock_item`.
- **FR-144-AC-5**, by `the_abstract_initial_states_are_part_of_the_obligation`.
- **FR-140/141/142/145 integration.** The refinement still checks, and its replay and settlement run through `check_refinement`, `open_systems` and `settle_refinement_safety`. AC-5's `InitialNotAbstract` refutation goes through replay.
- **FR-124.** Deadlock items, opt-out.

**What is covered:**
- The kind and form of the record, including the `liveness: false` variant.
- That the refinement's node holds exactly one record.
- The concrete-only deadlock item, with opt-out on `Impl` and none on `Spec`.
- AC-5's distinct obligations, plus deduplication of a repeated request.

**Mutations killed by existing tests:** without the equal-record arm, `two_refinements_of_one_meaning_refuse_naming_both` fails.

## Verdict

**Changes requested for FND-001.** FND-002 is low.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | AC-1's "routes to the explicit-state provider" is shown with only EN-1 registered. With EN-6 also registered, the untimed `CasRefinesCounter` record's candidates are [en-1, en-6], but FR-144 sends an untimed subject to explicit-state and only a `time dense` one to EN-6. The new `RequestItem::property_form` is read by no production path. | qsl-analyze/tests/it/refinement_request.rs:79-100; qsl-route/src/request.rs:107-115 |
| FND-002 | low | No test covers FR-124 when a temporal item and a refinement item are over the same concrete subject, and a different model holds `terminal any`. That is the case where SR-1751001 FND-001 shows the item depends on whether the refinement item is present. | qsl-analyze/tests/it/refinement_request.rs:128-150 |

## Dispositions

Round 1, at 20b4a7c8b. Every finding is fixed:
- SR-1751001 FND-001: the deadlock opt-out reads the subject's own models through `models_of`. With `terminal any` on `Spec`, temporal-only, refinement-only and both each give 1 item; with it on `Impl`, each gives 0.
- SR-1751001 FND-002: `located`'s doc is restored.
- SR-1751002 FND-001: fixed in #1753. With EN-1 and EN-6 registered, the untimed record's candidates are `[en-1]`.
- SR-1751002 FND-002: `a_temporal_and_a_refinement_item_share_the_concrete_deadlock_item` covers it.

Verdict: mergeable.
