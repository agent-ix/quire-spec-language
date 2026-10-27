---
id: SR-1690002
title: "gap analysis of PR #1690 (QSL-1005a2) against FR-159-AC-3/4/5, FR-160 and FR-430-AC-3"
type: SpecReview
analysis: gap-analysis
date: 2026-10-27
scope: "agent-ix/quire-spec-language; PR #1690 git diff origin/qsl-1005a-zone-symmetry...7f9a8e608; FR-159-AC-3 (second half), AC-4, AC-5; FR-160 ReductionKind; QSpec FR-430-AC-3 negotiation half; TC-583"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-159
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-160
    type: reviews
---
# SR-1690002: gap analysis of PR #1690 (QSL-1005a2)

## Summary

Ticket: QSL-1005. Head reviewed: 7f9a8e608. Method: each AC checked against its tests and the code paths, and the gates run. Plan completion: not assessed.

| Requirement | Measured |
| --- | --- |
| FR-159-AC-3, second half | **Partial.** `restrict_to_reductions` yields `missing_reduction = PartialOrder` and routes `symmetry` to the candidate, but only when a test calls it. No negotiation path applies it (FND-001). |
| FR-159-AC-4 | Identity half met: the text matches exactly. The "no exploration statistics after `ReductionNotPreserving`" half is not asserted by this PR, and `ModelCheckOutcome::Reduction` carries `stats` (FND-003). |
| FR-159-AC-5 | Manifest half met: EN-1 has the 3 labels, and the schema validates in place and refuses `state-constraint`. The routing half holds only through `restrict_to_reductions` called directly (FND-001). |
| FR-160 `ReductionKind` | Met: it moves to qsl-foundation with the one spelling `constraint`. |
| FR-430-AC-3, negotiation half | EN-6 advertises `symmetry` only. The negotiation is FND-001. |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | **No negotiation path filters by the selected reductions.** `Registry::restrict_to_reductions` and `RequestItem::reductions()` are called only from tests (`reduction_manifest.rs`, `reduction.rs`). The library's own candidate computation (`request::candidates_of`, `items_with_evidence`) never reads an item's reductions. Nothing turns `CandidateSet::missing_reduction` into the V-8 `unsupported-requested-capability` record. FR-159 says negotiation "SHALL route an item that selects reductions only to a candidate advertising every one of them". As built, that holds only for a caller who remembers to compose the two calls. Fix: apply the filter where an item's candidates are computed, for example a `candidates_for_*` variant that takes the item's reductions or a step in the writer that has them, and settle the V-8 from it. Test it through that path. | qsl-route/src/lib.rs:1514-1542; qsl-route/src/request.rs:110-116,706-733; qsl-analyze/tests/it/reduction_manifest.rs:58-80 |
| FND-002 | low | **The schema test passes when `QSPEC_DIR` is unset.** `en1_manifest_entry_validates_against_the_backend_provider_schema` prints "skipped" and returns `ok`, so in any environment without the variable FR-159-AC-5's schema half is a silent pass. Make it fail, or mark it `#[ignore]` and run it in the gate that sets `QSPEC_DIR`. | qsl-analyze/tests/it/reduction_manifest.rs:227-231 |
| FND-003 | low | **FR-159-AC-4's second sentence is untested.** It reads "After `ReductionNotPreserving`, the outcome records no exploration statistics", and the PR traces the test to AC-4. `ModelCheckOutcome::Reduction` has a `stats: RunStats` field, and no test asserts that it is zero or empty after `ReductionNotPreserving`. Add the assertion. | qsl-replay/src/model_check/outcome.rs:165-170; qsl-analyze/tests/it/reduction.rs:790-837 |

## Verdict

**Not mergeable together with SR-1690001.** FND-001 here is a real gap in AC-3's second half and AC-5's routing half.

## Dispositions, round 1 (head c400156b7)


Round 1. Fix c400156b7.

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | `items_with_reductions` restricts each `temporal-satisfaction` item's candidates in the writer's real path. `RequirementItem::unsupported_reduction` names the V-8 reduction. `the_request_writer_restricts_each_item_by_its_reductions` covers it: EN-6 alone with POR gives V-8 `PartialOrder`; adding EN-1 routes to EN-1 only; symmetry keeps both. |
| FND-002 | fixed | Without `QSPEC_DIR` the schema test panics ("QSPEC_DIR names the QSpec checkout...") and fails. I ran it with the variable unset. |
| FND-003 | fixed | After `ReductionNotPreserving{PartialOrder, TP-2}`, `stats == RunStats::default()` is asserted. |
| FND-004 (new) | low | Only `temporal-satisfaction` items are restricted. Details below. |

**FND-004: restricting only `temporal-satisfaction` items.** This matches EN-1, which advertises `temporal-satisfaction` only, and FR-382's "a model-check request item carries `reductions`" for the claims and derived items. But FR-159's preservation table also covers a refinement item: POR is false for both halves, and symmetry is true. A refinement item in a request that selects symmetry is routed "as if none were selected", so a candidate that does not advertise symmetry could run it unreduced. That breaks FR-382's "never answers a reduced request by running unreduced". Not blocking. Either restrict every item kind the request's reductions apply to, or state in FR-159 / FR-382 which item kinds carry the selection. Refs: qsl-route/src/request.rs:703-709.

**Verdict: mergeable.**

Gates at c400156b7 (locked-build, jobs 4, `--all-features`, debug info off, QSPEC_DIR=qspec-e3d48d76):
- fmt: exit 0.
- qsl-analyze `reduction` / `partial_order` / `quotient` / `zone_` / `hyper_` / `statistical_`: 362/362.
- qsl-route: 18 + 77.
- root `family_outcome_layering` / `temporal_negotiation` / `request_builder`: 20/20, TC-390 included.
- clippy on the four crates with `-D warnings`: exit 0.

+++ [reviewer data]

```yaml
dispositions:
- fnd: FND-001
  outcome: fixed
  after_excerpt: '} else if requirements.kind() == Capability::TemporalSatisfaction { registry.restrict_to_reductions(candidates_of(requirements, evidence, registry, named), &selected)'
- fnd: FND-002
  outcome: fixed
  after_excerpt: '.expect("QSPEC_DIR names the QSpec checkout whose schema this reads");'
- fnd: FND-003
  outcome: fixed
  after_excerpt: 'assert_eq!(*stats, RunStats::default());'
findings:
- fnd: FND-004
  severity: low
  confidence: medium
  method: gap-analysis
  check: coverage
  artifact_id: FR-159
  path: qsl-route/src/request.rs
  lines: 703-709
  finding: 'Only temporal-satisfaction items are restricted by the request''s reductions; FR-159''s preservation table covers refinement items too, which are routed as if none were selected. Restrict every kind the selection applies to, or state in FR-159/FR-382 which kinds carry it.'
```

+++
