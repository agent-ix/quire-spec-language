---
id: SR-1813001
title: "PR #1813 (QSL-1235) code review: QSpec FR-368-AC-14, AC-17, FR-429, FR-438 protocol-subject derived items"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; qsl-route deadlock.rs, model_request.rs; qsl-semantics check/mod.rs, protocol_clause; qsl-foundation provenance.rs; qsl-analyze tests at 3271b6a80"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-211
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-224
    type: reviews
---

# PR #1813 code review

## Summary

Head 3271b6a80, base terra-qsl-integration. Checked against QSpec 9450676f.

**What holds.**
- **Anchor (FR-368-AC-14).** A protocol subject's derived items are anchored at the checked protocol clause's node, at its `declaration` occurrence with ordinal 0. The test confirms the emitted package's source map holds that anchor. The model subject's items keep the `model_root` anchor.
- **Order (FR-368-AC-17).** Caller items come first in caller order. Derived items are then grouped by subject in first-occurrence order, each group as `deadlock-freedom` then `race-freedom`. A protocol subject and a model subject over the same model are two subjects.
- **Item shape (FR-438).** The race-freedom item is `ReachableInvariant`, unbounded `temporal-satisfaction`, with candidates EN-1 and EN-6. Its identity carries the resolved models. A protocol clause's `terminal any` opts the subject out of `deadlock-freedom` (FR-429).
- **Removed helpers.** `protocol_deadlock_freedom_items`, `race_freedom_items`, `DeadlockFreedomItem`, `RaceFreedomItem` and `ProtocolMemoryItem` are gone. No code caller remains, and no compatibility path is left behind.
- **`CheckedProtocol::node`.** It is filled from `lowering.protocol_key` by the same protocol index the refinement loop uses.
- **Unanchored fixtures are honest.** A probe printed each fixture's clause node and request:
  - The FR-224-AC-3 message-passing shape has no node only because its unit selects no protocol profile. Its derived items are counted from `unanchored` with cause `NoProtocolNode`.
  - The shape cannot simply be profiled: under the protocol profile it is refused with `VisibleEntryNotAnExpression`.
  - `two_plain_stores`, which does select the profile, anchors as expected:
    - under `tso` and `ra`: `deadlock-freedom` and `race-freedom`;
    - under `sc`: `deadlock-freedom` only.
  - The atomic shape yields no race item.
  - `derived_subjects` counts written and unanchored items together, so no count went vacuous.
  - `the_repaired_fill...` now uses two distinct occurrences, so the per-subject dedup is really exercised.

The coder's two open questions are with the planner and are not findings: whether a protocol over a timed model is a timed subject, and race-freedom in `derived_model_item`.

**Gates.** All exit 0, run with `nice -n 10`, jobs 4 and threads 4: fmt; qsl-route (98 tests); qsl-foundation lib (52); qsl-semantics lib (817); qsl-analyze `it` (1060); clippy `-D warnings` on the four touched crates with all targets; `make check-spec-validation`.

## Verdict

Mergeable. One low finding.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | **`SubjectReading::merge` silently keeps the first protocol reading for one `S`.** QSpec FR-425 makes each outermost `parallel`'s resolved memory model part of the protocol subject, and the removed `race_freedom_items` deduped on (subject, models). Now, when two caller items share `S` but differ in resolved models or clause node, the first item's `race` and `node` win. The second item's models get no race-freedom item and no diagnostic. Either key the grouping on the resolved models too, or document and `debug_assert!` that `S` includes them and that the readings agree. No production caller exists yet (QSL-1243 adds one), so nothing is wrong today. | QSpec FR-425 "Protocol subject", FR-438, FR-368-AC-17 |
