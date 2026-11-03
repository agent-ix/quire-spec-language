---
id: SR-2026110215411
title: "QSL-1473 PR #2151 manual acceptance gap analysis"
type: SpecReview
analysis: gap-analysis
review_set: subset
scope: "agent-ix/quire-spec-language@00b1763b11535c7215b64ef25c285e5dc4a2b885; QSL-1473 acceptance; qsl-analyze/tests/it/fill.rs, qsl-analyze/tests/it/rc11_oracle.rs"
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-223
    type: reviews
---

## Summary

Ticket: QSL-1473. Frozen head: `00b1763b11535c7215b64ef25c285e5dc4a2b885`. Plan completion: not assessed.

Quoin's `gap-analysis` command is unavailable (`command gap-analysis not found`). A repository-wide `quire matrix --scope . --format tsv` emitted module diagnostics but produced no matrix rows during a 30-second attempt, so computed coverage was not available. Per reviewer procedure, performed a manual acceptance-criteria-to-test/source reconciliation. AC-1's caller pin and N-worker assertion are present; AC-3's exact totals and forbidden-outcome oracle remain; existing LT-12/LT-13 tests are unchanged; no cap was added. AC-2's required interleaved 1/2/4/all-cores baseline/candidate results, `other_busy`, lane, profile method, and scaling analysis are not present in this PR evidence. The PR description explicitly leaves those measurements pending. This is a remaining ticket acceptance gap, not a source-code finding in the worker-configuration patch.

## Verdict

**CONDITIONAL** — merge of this enabling code patch can be reviewed independently, but QSL-1473 cannot be closed until AC-2 is completed and recorded on the ticket.

## Coverage

- QSL-1473 AC-1 (examined): “The worker count is settable by the caller and honoured by the explorer; a test proves a run with N workers uses N workers.” `reachable_with_workers` accepts a `NonZeroUsize`; `expand_level` spawns the bounded count; LT-16 asserts the reported spawned-worker maximum equals the requested limit. Coder reports a focused N=3 run passed; not independently rerun by reviewer.
- QSL-1473 AC-2 (examined): “Results table for 1 / 2 / 4 / all cores, each with `other_busy`, the lane, and the profile method. Do not describe overlapping runs as isolated.” No such result table or profile is present in PR #2151 or the ticket evidence inspected. PR description says interleaved scaling measurements and follow-up remain pending.
- QSL-1473 AC-3 (examined): “LT-16 still reports exactly 7,607 states and 19,574 edges with the oracle assertion unchanged, at every worker count. LT-12 and LT-13 keep their assertions.” LT-16 assertions remain in `rc11_oracle.rs`; LT-12/LT-13 code is not changed. A single focused N=3 run is reported by the coder, not independent evidence for all requested counts.
- QSL-1473 AC-4 (examined): “No state, depth or input-size caps are introduced.” The diff changes worker scheduling only and introduces no state, depth, or input-size cap.
- `qsl-analyze/tests/it/fill.rs` and `qsl-analyze/tests/it/rc11_oracle.rs` (examined): source/test bindings for the worker configuration and LT-16 oracle.
- `FR-223-AC-8` and `TC-668` (context_only): normative LT-16 outcome and owning test case.

Full computed matrix and Quoin gap-analysis were unavailable; this is a scoped manual check only. No aggregate gate, tests, or performance measurement was run by this reviewer. No applicable AssuranceProfile was found. Plan completion: not assessed.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | The required 1/2/4/all-cores interleaved baseline/candidate results, with `other_busy`, lane and profile method, plus a profile-based scaling explanation, are absent. Without these results QSL-1473 AC-2 and the ticket's scaling conclusion remain unverified; the PR description confirms this work is pending. | QSL-1473 AC-2; qsl-analyze/tests/it/rc11_oracle.rs:167-235 | correct-requirement-no-evidence |

## Disposition

| Finding | Outcome | Evidence and limitation |
| --- | --- | --- |
| FND-001 | Verified fixed | The required 1/2/4/14-worker baseline/candidate measurements and profile-based scaling discussion are recorded in [QSL-1473 measurement comment 51387245](https://linear.app/agent-ix/issue/QSL-1473/make-lt-16-explorer-worker-count-configurable-and-measure-multi-core#comment-51387245), and summarized in PR #2151 at head `126da8f`. Each run passed the exact 7,607-state / 19,574-edge oracle. All measurements had `other_busy=1`, so they are contended and do not support an isolated performance attribution. The reviewer disposition report for PR #2151 records FND-001 as verified fixed. |
