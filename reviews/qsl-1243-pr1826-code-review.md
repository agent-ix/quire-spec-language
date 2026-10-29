---
id: SR-1826001
title: "PR #1826 (QSL-1243) code review: protocol EN-1 analyze item, QSpec FR-429, FR-438, FR-425, FR-368-AC-17/AC-20"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; qsl-analyze analyze.rs, analyze/items.rs, zone_check/certificate.rs; qsl-route deadlock.rs, model_request.rs at d690d8299 (commits 80c45f266, 5978331ae, d690d8299; the #1813 commits were reviewed in SR-1813001)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-281
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-215
    type: reviews
---

# PR #1826 code review

## Summary

Head d690d8299, base terra-qsl-integration. Checked against QSpec 69047b25.

**The `derived_protocol_item` limits.**
- It drops `max_depth`, since a derived item is an exhaustive search.
- It applies the caller's `max_states` and `max_transitions` to the run.
- It applies `max_store_buffer`, `max_messages` and `max_live_instances` to both the run and the certificate check. An unset one keeps EN-1's default.
- `protocol_item` reads `request.limits` after this adaptation, so the certificate check uses the same graph limits as the run.
- It reads no `max_clocks` and no solver limits. Neither applies, because a protocol subject is never timed and EN-1 has no SMT step.
- A probe confirmed both behaviours are tested:
  - keeping `max_depth` fails `a_protocol_subjects_derived_items_settle_through_analyze_on_en1`;
  - not passing the caller's `max_messages` to the certificate fails `a_protocol_derived_items_caller_limits_reach_en1`.

**`EngineReport::Protocol`.** It is settled with `to_terminal`. A refuted result without its reproduced counterexample becomes `failed`, as `Model` and `Zone` do. `AnalyzeRecord.protocol_check` carries the record, and `outcome_item` reads the protocol counterexample's `UndefinedEvaluation` kind as it reads the model counterexample's.

**`CertificateItem::Other` for `ProtocolAuthored`, `ProtocolDeadlockFreedom` and `RaceFreedom`.** This is safe.
- EN-6's `check_timed_model` admits only `Authored`, `DeadlockFreedom` and `TimeLockFreedom` obligations. Anything else is refused with `ObligationMismatch` before a search.
- The zone certificate checker refuses `ItemIdentity::Other` (`zone_certificate.rs` line 807).
- No other match on `ObligationKind::Authored` exists that `ProtocolAuthored` now bypasses.

**Splitting by subject (FR-425).** Readings are grouped by caller subject and resolved models. Each set of models gets its own deadlock-freedom item (`ProtocolDeadlockFreedom { models }`) and race-freedom item, in first-occurrence order (FR-368-AC-17). A model subject and a protocol subject with the same caller subject are still kept apart. `ProtocolAuthored { occurrence, models }` makes one clause under two sets of models into two items. This resolves my #1813 observation.

**Gates.** All exit 0, run with `nice -n 10`, jobs 4 and threads 4:

| Gate | Result |
| --- | --- |
| fmt | pass |
| clippy `-D warnings` (qsl-route, qsl-analyze, all targets) | pass |
| qsl-route | 98 passed |
| qsl-analyze lib | 94 passed |
| qsl-analyze `it` | 1067 passed |
| `make check-spec-validation` | pass |

FR-438-AC-1's message-passing shape is QSL-1248 and is not counted here.

## Verdict

Mergeable. One low finding.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | **`derived_protocol_item` checks the item kind but not the subject's resolved models.** It receives only the `RequestItem`, never its obligation. Since 5978331ae, one caller subject can have several derived items, one per set of resolved models. A run or certificate request over the `tso` subject passed for the `ra` item would therefore be settled at the `ra` item's index without complaint. Nothing ties the run's subject to the certificate request's subject either. The fix is to take the item's `ItemObligation`, compare its `ProtocolDeadlockFreedom` or `RaceFreedom` models with `request.subject`'s and `certificate.subject`'s resolved models, and refuse a mismatch as `OtherItem`. This is caller wiring: `derived_model_item` has the same shape. | QSpec FR-425, FR-368-AC-17, FR-281 |
