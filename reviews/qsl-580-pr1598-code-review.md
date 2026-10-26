---
id: SR-1598001
title: "code-review and rust-review of PR #1598 (QSL-580c, FR-232 time-lock item, FR-236 delays)"
type: SpecReview
analysis: code-review
date: 2026-10-26
scope: "agent-ix/quire-spec-language; PR #1598 git diff ddeff5dd6...cb95a247a: qsl-route/src/deadlock.rs, qsl-eval/src/simulation/model/timed/over_time.rs, qsl-replay/src/model_check/{counterexample.rs,replay.rs,hyper_replay.rs,hyper_counterexample.rs}, qsl-analyze/src/model_check/* (delay: None), tests"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-232
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-236
    type: reviews
---
# SR-1598001: code-review and rust-review of PR #1598 (QSL-580c, FR-232 time-lock item, FR-236 delays)

## Summary

Ticket: QSL-580. Coder note on wire and identity: `TemporalCounterexample` and `ModelStep` have no in-repo canonical or wire encoder. They are in-memory `Eq` structs, and `plugin_read.rs` already accepted an optional `delay` on the plugin wire. Every existing engine sets `delay: None` and `final_delay: None`, so the value, equality and replay of every untimed counterexample are unchanged. The only observable change is `FamilyPayload::measured_bytes`, which grows by `size_of::<Option<Rational>>()` per step and per counterexample (FND-003). `replay_prefix` and the hyper `reexecute` now refuse a delay with `UntimedDelay`. Every model replay path, including refinement and protocol replay and the lasso loop (which lies inside `prefix`), goes through these two, so no delay is silently ignored. Coder note on `#[allow(clippy::large_enum_variant)]` on `HyperStep`: it carries a reason, and the same allow is used for the same reason in qsl-semantics (typing.rs, lowering.rs, conformance.rs and others), so it is accepted. Coder note on the fixture's `reset` operation: ADR-026 §11's `Rpc` declares `operation Call::reset` with exactly that pre and post, so it is faithful to the spec's fixture, and FR-232-AC-2's "`(Replied, x = 2)` is not terminal" depends on it. Interval arithmetic (`DelayInterval::of`, `meet`, `covers`, `within` with resets) was traced, including negative edges, which `meet` with `all()` normalises. Focused tests at the stack head pass.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `weakly_fair` reads enabledness only over the delays between represented positions, plus delay 0 at the last. For a loop of one position, such as FR-231's idle tail (one stutter step after a positive delay), no window exists. A constraint whose guard becomes enabled during the stutter delay (for example `x >= 1` from `x = 0`) and stays enabled is then judged fair, though it is continuously enabled and never taken. The `TimedBehaviour` API also cannot represent the stutter delay to judge it. Traced, not reproduced. | qsl-eval/src/simulation/model/timed/over_time.rs:373-404 |
| FND-002 | low | `time_lock` returns `None` for a non-local time-lock from which a positive delay is still admissible, such as `Stall`'s `(s0, x = 1/2)`. It decides only the states where time is already stopped. The doc comment and the PR body say so. Recorded so that FR-240's search, not this helper, is relied on for TD-1. | qsl-eval/src/simulation/model/timed/over_time.rs:296-340 |
| FND-003 | low | Adding `delay: Option<Rational>` grows `size_of::<ModelStep>()`, and so the `measured_bytes` charged for every untimed counterexample. A witness-bytes budget can now trip on a counterexample it admitted before. This is not an identity change. Box the delay, or accept it and say so. | qsl-replay/src/model_check/counterexample.rs:254-271,55-62 |
| FND-004 | low | `is_timed` tests the whole package (`!time_sources().is_empty()`), not the subject's model. In a package with a timed and an untimed model, a subject over the untimed model gets a time-lock-freedom item. `opts_out` has the same package-level shape. Also, neither `time_lock_freedom_items` nor `deadlock_freedom_items` is called by a production request writer yet, so FR-232-AC-1 is asserted on the helper only. | qsl-route/src/deadlock.rs:230-257 |

## Verdict

**Mergeable**, with FND-001 fixed or deferred to the lasso/liveness slice (QSL-581c) with a ticket. No AC claimed in this PR exercises a one-position loop. No high finding. Untimed counterexample values and replay are unchanged, and delays are refused where they cannot be checked. Rust lane: exact arithmetic, iterative exploration with an explicit budget in `time_lock`, and the clippy allow carries a reason that matches repo precedent.

## Coverage

Methods: code-review with the rust-review lane. Gap analysis is SR-1598002.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-005 | medium | Regression from the FND-001 fix: `weakly_fair` shifts each unreset clock past the model's largest constant with `while *value <= ceiling { *value = value.add(&total) }`, one exact addition per loop repetition, with no meter or cancel. Reproduced on bd68b877a: an idle tail with stutter delay 1 under a guard `x >= 1000000000000 ms` did not return within 60 s. FR-230 admits constants up to `10^40 ms` (FR-230-AC-4), so a valid model hangs the check. Compute the repetition count directly, `k = floor((ceiling - value) / total) + 1`. | qsl-eval/src/simulation/model/timed/over_time.rs:455-470 |

## Dispositions

Round 1, against #1598 fix commit f2efd8da7 on the restacked head bd68b877a (is_timed fix carried in restacked 3880da60a; `git range-diff ddeff5dd6..cb95a247a origin/main..3880da60a`).

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | - |
| FND-002 | accepted-no-change | Documented helper limit, as the finding recorded: `time_lock` decides only states where time is already stopped, and its doc comment hands the rest to the zone engine. #1599's FR-240 divergence test (zone_check/divergence.rs) is the TD-1 decision procedure. |
| FND-003 | fixed | - |
| FND-004 | fixed | - |

Round 2, against round-2 commit 570a62e05 (head).

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-005 | fixed | - |
