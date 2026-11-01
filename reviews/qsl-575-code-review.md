---
id: SR-202610310139
title: "Code review — QSL-575 unbounded exact probability solver"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; QSL-575; PR #2014 at 16ad281d627c55cc5daf075234e0cc7469787633; qsl-analyze/src/exact_probabilistic.rs, qsl-analyze/src/exact_probabilistic/unbounded.rs; FR-198, FR-199, TC-633, TC-634"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-198
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-199
    type: reviews
---

## Summary

Ticket: QSL-575. Reviewed PR #2014 at frozen head `16ad281d627c55cc5daf075234e0cc7469787633`, covering the new unbounded reachability, expected reward, and long-run solver plus FR-198/FR-199 tests. Rust review found two incorrect-result paths and one malformed-product panic; no CI workflow files changed and no applicable AssuranceProfile was found.

## Verdict

**FAIL** — minimum expected reward can be understated as finite, MDP long-run optimization can miss an optimal proper end component, and `decide_until` can panic on an invalid edge source before returning the typed invalid-state error.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Minimum expected-reward policy improvement treats a nonterminating successor as zero, so it can select a policy with infinite expected cost and report a finite lower value. | qsl-analyze/src/exact_probabilistic/unbounded.rs:463-516, 1089-1220 |
| FND-002 | high | Long-run MEC search discards policies that have a proper sub-MEC recurrent class, so it can report 1/2 where the maximum is 1. | qsl-analyze/src/exact_probabilistic/unbounded.rs:1655-1703 |
| FND-003 | medium | `product_with_blocked` indexes by caller-supplied `edge.from` before validating the product, so `decide_until` panics instead of returning `InvalidState` for an out-of-range source. | qsl-analyze/src/exact_probabilistic/unbounded.rs:199-201 |

## Coverage

Examined FR-198-AC-1 through AC-5, FR-199-AC-1 through AC-3, TC-633 and TC-634, both changed Rust files, existing probabilistic product grouping/build behavior, and workspace gate configuration. The new tests carry the expected trace tags for all eight acceptance criteria. The timed observation portion of FR-198 §13a / AC-2 is explicitly retained for QSL-577 and is not claimed as supported or refused by this PR. FR-198-AC-5's test checks the solver value and action index but not the required settlement/replay of the refutation evidence; see the separate gap-analysis artifact.

Coder-reported evidence at frozen head: 13 focused tests, 118 qsl-analyze library tests, no-default `cargo check`, all-target no-default Clippy, and no-default build passed. Reviewer ran `cargo fmt --check` successfully. A reviewer `cargo test -p qsl-analyze` overlapped the coder's uncommitted fix round and exited 101 (119/120); the added expected-reward regression failed with `MissingAction`, so that run is not evidence for the frozen PR head. The review-time matrix commands stalled and were stopped; see gap analysis.

## Review Details

### Rust review

The changed surface is Rust in `qsl-analyze`. No unsafe code, async tasks, or wire format changes were introduced. Public solver results are documented. The three findings below are concrete soundness/boundary defects and block the frozen head.

1. For FND-001, an MDP with a finite `good` action to the target at cost 10 and a `bad` action that reaches a positive-cost nonterminating state with probability 1/2 has minimum expected cost 10. The code sets globally infinite states to zero in the evaluation vector, then policy improvement can choose `bad` on that zero-valued continuation. Compare all outgoing choices against an extended-rational value that preserves `+infinity`, and add a regression.
2. For FND-002, in a maximal MEC with A-stay, A-to-B and B-to-A, predicate true only at A, the proper sub-MEC `{A}` has fraction 1. Requiring the selected policy to be strongly connected across the whole maximal MEC rejects the policy that stays in A and reports 1/2. Optimize over attainable recurrent subcomponents and retain the transient steering policy.
3. For FND-003, `ProbProduct` and its edge vector are public. A source-state ID from another larger product can be placed on an edge in a smaller product. `decide_until` builds a blocked-state vector and filters edges by unchecked indexing before `Graph::new` validates IDs. Validate first or use checked access and return `UnboundedError::InvalidState`.

No cargo-deny or full all-feature reviewer gate was run during this initial pass because the candidate changed while the review was in progress; the coder's frozen-head verification above is recorded separately. No workflow was narrowed or removed.

## Findings Detail

### FND-001 — Preserve infinity during minimum reward policy evaluation

`decide_expected_reward` marks target and objective-infinite states with `Some(0)` in `fixed`, then `policy_iteration_reward` evaluates and improves policies using those zero values. Repro: state 0 has a `good` action to target with probability 1 and reward 10; a `bad` action has probability 1/2 to target and 1/2 to state 2, both reward 0; state 2 self-loops with reward 1. The minimum is 10, while `bad` has expected reward `+infinity`. The current policy evaluator can see the state-2 continuation as zero and select `bad`, yielding a false finite minimum. This violates FR-198-AC-2's exact minimum expected-reward result and FR-198's `+infinity` rule.

### FND-002 — Include optimal recurrent proper sub-MECs

`optimize_mec_fraction` enumerates internal deterministic policies but accepts a candidate only when its induced graph is strongly connected across every state of the maximal MEC. Repro: A has actions `stay` (A to A with probability 1) and `to_b` (A to B with probability 1); B has `to_a` (B to A with probability 1); predicate is true at A only and weight is 1. `{A,B}` is a maximal MEC and `{A}` is a proper end component. The maximum value from either initial state is 1: from B, go to A, then stay. The policy that makes B transient and A the unique recurrent class is filtered out, leaving only the full-cycle policy with value 1/2. This violates FR-199-AC-2's scheduler extremum behavior.

### FND-003 — Validate edge source before filtering `until` products

`product_with_blocked` evaluates `blocked[edge.from.index()]` before `Graph::new` performs its state-reference validation. Repro: obtain `ProbStateId(1)` from a valid two-state product, put it as the source of an edge in a one-state public `ProbProduct`, and call `decide_until` with one-element predicates. The blocked vector has length one; the public API panics at index 1 rather than returning `UnboundedError::InvalidState`. Validate input edges before filtering.
