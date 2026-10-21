---
id: "SR-12266"
title: "gap-analysis of QSL-580 remainder: timed lasso evaluation (FR-231) and FR-235 V-8 punctual settlement (PR #1153)"
type: "SpecReview"
analysis: "gap-analysis"
scope: "agent-ix/quire-spec-language@63a8bd86a9711dcec28fbe5fff6aa83558e94c70; PR #1153; git diff origin/main...HEAD; spec FR-231 (Inputs, Evaluation), FR-235 (V-8, AC-4), ADR-026 CT-2, CT-3, DF-5, TV-1; QSpec FR-331, FR-416; QSL-580"
review_set: "subset"
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-231
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-235
    type: references
---

## Summary

Ticket: QSL-580. This is a planless gap analysis of PR #1153. It checks FR-231's "Evaluation" lasso clause and FR-235's V-8 row and AC-4 against the tagged tests and sources, reading ADR-026 CT-2, CT-3 and DF-5 and QSpec FR-331 and FR-416 for context. The FR-231 lasso clause is backed by four FR-level tests with meaningful oracles. FR-235-AC-4's tag overclaims: no settlement path exists. The lasso tests miss the equal-stamp loop where SR-12265 FND-001 lives, as well as the meter, overflow and undefined branches.

## Verdict

**PASS for this method, no high finding.** The PR as a whole is blocked by SR-12265 FND-001. FND-001 and FND-002 below are medium and should be fixed in this PR.

### Declared blocked items judged (PR body, untrusted data, re-measured)

| Item | Judgment |
| --- | --- |
| Wire `locus` of `punctual-interval{locus}` | Blocked, sound. QSpec FR-331 makes the locus a checked-package `{node, occurrence}`, and S3 has only a source span. The cause must not be wired into settlement until it lands (SR-12265 FND-002). |
| FR-235 V-1, V-4, V-6, V-7, V-10 and AC-1..AC-3, AC-5 | Blocked, sound. Each needs the zone engine (FR-239), replay (FR-237) or the certificate checker (FR-245), and none of these exists in the tree. |
| `StopwatchRequired` has no producer | Sound. Nothing in S3 detects stopwatches (FR-248's scope). |
| FR-233 binding key (QSpec FR-252), FR-230 facet gate | Sound. These were carried from SR-12250 FND-002. No production path builds a timed clause, so `TimedTrace::lasso` is also reached from tests only. |
| FR-370 timed emission | Sound. It is out of this slice. |

## Coverage
Plan completion: not assessed

- FR-231 "Evaluation" (lasso clause): four tests tagged `FR-231`/`TC-686` in qsl-eval/tests/it/timed_trace.rs. `a_timed_lasso_grows_its_stamps_by_the_loops_delay` checks a punctual window landing on stamps `1, 3, 5, …` at positions 0, 1, 4 and 5, and contrasts the prefix's `Pending`. `an_unbounded_claim_is_decided_on_a_timed_lasso` checks `always eventually` holding and violated, plus `until`. `a_past_operator_reads_the_unrolled_loop` checks `always once` at 0, 1 and 7. `a_timed_lasso_refuses_a_loop_that_is_not_time_divergent` checks each refusal and the boundary at the last stamp. The oracles are hand-checkable and can fail. Bindings correct at FR level, since the clause has no AC id.
- FR-235-AC-4: `a_punctual_item_settles_unsupported_naming_the_interval` (qsl-replay proof_result unit test). Binding wrong (FND-001).
- Matrix: not run (`quire matrix` did not finish on this machine in the last round). Tags were inventoried directly: `FR-235-AC-4` has 1 binder, and `FR-231` has 4 FR-level binders.
- Reverse gap: `From<&UnsupportedItem> for UnavailabilityCause` has no production caller. It is reached only by its test. No stub, no ceremony, no vendored file.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `a_punctual_item_settles_unsupported_naming_the_interval` is tagged `FR-235-AC-4`, but it only unit-tests `From<&UnsupportedItem>`, `as_str` and `basis`. AC-4 requires FR-234-AC-3's punctual claim to settle `unsupported`/`PunctualInterval`, and it also has a `Stall` part and a zone-search part. The `From` impl has no production caller, so no disposed item reaches a terminal record. SR-12250 FND-004's settlement gap is still open. The test name and doc also claim "naming the interval", but the wire record drops the span. Fix: drop the `FR-235-AC-4` id (keep `TC-690`, tag `FR-235`) and rename the test, or wire the request writer's disposition into settlement and test that path. | qsl-replay/src/proof_result.rs:1423-1438, 375-383 |
| FND-002 | medium | Untested lasso branches: (a) a loop with equal stamps, where SR-12265 FND-001's wrong verdict lives; (b) a meter stop during `unrolled` or the tail scan (`Incomplete`); (c) `TraceTooLong` when the copy count overflows `u64`; (d) an undefined atom on a lasso; (e) a bounded `until` whose guard spans the tail. Add at least (a), (b) and (d). | qsl-eval/tests/it/timed_trace.rs:302-425; qsl-eval/src/temporal/timed.rs:329-372, 630-677 |

## Dispositions

Round 1 was reviewed at 8550a71f195b76a9aa5fb3d8b0281cf156e75bd6 (fix commit 8550a71f1). The focused run `cargo test -p qsl-eval --test it timed_trace` passed 13 of 13.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 8550a71f1. after: `/// FR-235 V-8, cause only: ... No settlement path exists yet, so this backs no acceptance criterion.` and `#[trace("TC-690", "FR-235")]`. The tag is now honest. The fn name still reads `..._settles_unsupported_naming_the_interval`. That is a leftover naming nit and does not block. |
| FND-002 | fixed | 8550a71f1. Added tests: `a_tail_scan_reads_every_position_that_shares_a_stamp` (equal stamps, open lower end), `a_timed_lasso_stops_when_the_meter_runs_out` (Incomplete), `a_timed_lasso_refuses_an_unrolling_that_overflows` (TraceTooLong), `an_undefined_atom_on_a_timed_lasso_is_reported_at_its_position`, and `a_bounded_until_reads_its_window_in_the_tail`. The last one's witness lies in an unrolled copy, not the tail proper. Items (a), (b) and (d), which the finding required, are covered. |
