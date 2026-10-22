<!-- reviewer repo=agent-ix/quire-spec-language visibility=public quoin=0.28.3 module=spec-artifacts-process@b82e42e893612b4b4ee94127714b4577ee893502 id=SR-13545 method=spec-review/base lang=spec pr=quire-spec-language#1098 model=claude-opus-5-5 run=9F8790A4-6653-4A89-BBBF-6C1FC5C1A890 -->

SR file: `reviews/qsl-570-pr1098-spec-review-base.md` (SR-13545), head a53c86bc4

| FND | Severity | Check | Summary |
| --- | --- | --- | --- |
| FND-001 | medium | coverage | The two new FR-192 expansion-stop rows have no acceptance criterion and no TC-627 step |
| FND-002 | medium | coverage | The event-as-0/1 reading for a mean-of-fraction claim has no acceptance criterion |
| FND-003 | low | coverage | Regenerative max_draws and the TraceIndexRange exemption have no acceptance criterion |

Verdict: Two medium findings and one low. What holds: every claim in the PR body matches the code on qsl-570-impl. TraceIndexRange is raised only for Okamoto and SPRT. An event sample counts as 1 or 0 for a mean of a fraction. max_draws counts walk steps for the regenerative method, matching FR-190's draws. The new behaviour has no acceptance criterion.

+++ [reviewer data]

```yaml
clean: false
scope:
- id: FR-189-mean-of-fraction-bullet
  path: spec/functional/FR-189-decide-a-probabilistic-claim-by-statistical-model-checking.md
  role: examined
  excerpt: EN-4 SHALL decide a mean of a fraction by the fixed-sample method over the sample mean of the fraction. A request
    naming the sequential method for it SHALL be refused as `NotStatistical`, as FR-408 states. A sampler that returns an
    event sample for a mean-of-fraction claim SHALL be read as the fraction 1 when the event holds and 0 when it does not,
    and EN-4 SHALL not refuse it.
- id: FR-189-refusals
  path: spec/functional/FR-189-decide-a-probabilistic-claim-by-statistical-model-checking.md
  role: examined
  excerpt: '`TraceIndexRange` when `max_draws · m` overflows the trace index of Okamoto or SPRT (the regenerative test `j`
    uses trace index `j` alone, so the method never raises it), `ConfidenceBeyondPrecision` when `min(α'', β'')` has no normal-quantile
    bound for the regenerative method, `FractionForEvent` for a fraction a Bernoulli test cannot take'
- id: FR-191-limits-struct
  path: spec/functional/FR-191-bound-and-reproduce-a-statistical-run.md
  role: examined
  excerpt: 'pub max_draws: u64,        // samples drawn, counted or not, per test (steps taken, for the regenerative method);
    default 67_108_864 (2^26)'
- id: FR-191-budget-bullet
  path: spec/functional/FR-191-bound-and-reproduce-a-statistical-run.md
  role: examined
  excerpt: When a test has drawn `max_draws` samples (for the regenerative method, taken `max_draws` walk steps in total,
    as FR-190 counts steps as draws), or taken `max_cycle_steps` steps in one cycle, it SHALL stop the same way before the
    next draw or step.
- id: FR-192-expansion-limit-row
  path: spec/functional/FR-192-settle-a-statistical-result-on-the-measured-axis.md
  role: examined
  excerpt: '| `Stopped` by a sample expansion (`Expansion(ExpansionStop)`) that names a limit (`EvaluationMeter` or `MaxCandidates`,
    FR-126) | `incomplete` | `limit-reached{limit, value, setting}` with the stop''s limit, the value it was set to and the
    request setting that raises it (FR-126''s setting for that limit) | none | `Incomplete(LimitReached{limit, value, setting})`
    (ADR-018 V-7) | incomplete |'
- id: FR-192-expansion-unnamed-row
  path: spec/functional/FR-192-settle-a-statistical-result-on-the-measured-axis.md
  role: examined
  excerpt: '| `Stopped` by a sample expansion that names no limit and is not an internal failure | `incomplete` | `resource-exhausted`
    | none | `Incomplete(ResourceExhausted)` (ADR-018 V-7) | incomplete |'
- id: FR-192-AC-1
  path: spec/functional/FR-192-settle-a-statistical-result-on-the-measured-axis.md
  role: examined
  excerpt: 'Each outcome row maps exactly as the table states: an Accepted, a Rejected and an Undecided (`IndifferenceRegion`)
    `Completed` outcome; a `Stopped` naming `max_samples`, settling `Incomplete(LimitReached{limit, value, setting})` with
    setting `statistical.max_samples`; an `Unsupported` outcome with cause `NotMarkov`; a `NoTest` refusal settling `inconclusive`,
    `no-initial-state`; an `Internal` refusal settling `failed`.'
- id: FR-190-outputs
  path: spec/functional/FR-190-measure-a-long-run-fraction-by-regeneration.md
  role: context_only
  excerpt: 'FR-189''s `StatisticalOutcome`, whose verdict basis records `Regenerative{min_cycles}`, the cycle count as `samples`,
    the steps taken as `draws`, and `coverage: Asymptotic`.'
- id: PR-1079-code
  path: agent-ix/quire-spec-language@a9a693ac0 qsl-analyze/src/statistical.rs, qsl-eval/src/simulation/expansion.rs, qsl-analyze/src/model_check/state_graph.rs
  role: context_only
  excerpt: 'halted(): SampleHalt::Expansion(stop) => TestEnd::Stopped(StatisticalStop::Expansion(stop)); ExpansionStop { cause:
    CatalogCode, limit: Option<ExpansionLimit> }'
findings:
- fnd: FND-001
  severity: medium
  confidence: high
  method: spec-review/base
  check: coverage
  artifact_id: FR-192-expansion-limit-row
  related_ids:
  - FR-192-expansion-unnamed-row
  - FR-192-AC-1
  path: spec/functional/FR-192-settle-a-statistical-result-on-the-measured-axis.md
  lines: 100-101
  excerpt: '| `Stopped` by a sample expansion (`Expansion(ExpansionStop)`) that names a limit (`EvaluationMeter` or `MaxCandidates`,
    FR-126) | `incomplete` | `limit-reached{limit, value, setting}` with the stop''s limit, the value it was set to and the
    request setting that raises it (FR-126''s setting for that limit) | none | `Incomplete(LimitReached{limit, value, setting})`
    (ADR-018 V-7) | incomplete |'
  finding: 'The two new map rows have no acceptance criterion and no TC-627 step. FR-192-AC-1 lists the rows it tests, and
    an expansion stop is not among them, so nothing checks that a meter or max_candidates stop settles limit-reached with
    its limit, value and setting, or what an unnamed stop settles. Fix: extend FR-192-AC-1 (and TC-627) with an evaluation-meter
    stop naming its limit, value and setting, and an unnamed non-internal stop, as FR-149-AC-10 does for the certificate checker.'
- fnd: FND-002
  severity: medium
  confidence: high
  method: spec-review/base
  check: coverage
  artifact_id: FR-189-mean-of-fraction-bullet
  related_ids: []
  path: spec/functional/FR-189-decide-a-probabilistic-claim-by-statistical-model-checking.md
  lines: 150-155
  excerpt: EN-4 SHALL decide a mean of a fraction by the fixed-sample method over the sample mean of the fraction. A request
    naming the sequential method for it SHALL be refused as `NotStatistical`, as FR-408 states. A sampler that returns an
    event sample for a mean-of-fraction claim SHALL be read as the fraction 1 when the event holds and 0 when it does not,
    and EN-4 SHALL not refuse it.
  finding: 'The owner decision to read an event sample as fraction 1 or 0 for a mean-of-fraction claim, with no refusal, has
    no acceptance criterion and no TC-624 step. The code does this (statistical.rs:403-427), but a later change to refuse
    it, or to refuse FractionForEvent on the wrong side, would break no test. Fix: add an FR-189 AC (with a TC-624 step) where
    a sampler stub returns Event samples for a mean-of-fraction claim and the estimate equals the share of samples that hold.'
- fnd: FND-003
  severity: low
  confidence: high
  method: spec-review/base
  check: coverage
  artifact_id: FR-191-budget-bullet
  related_ids:
  - FR-189-refusals
  path: spec/functional/FR-191-bound-and-reproduce-a-statistical-run.md
  lines: 75-77
  excerpt: When a test has drawn `max_draws` samples (for the regenerative method, taken `max_draws` walk steps in total,
    as FR-190 counts steps as draws), or taken `max_cycle_steps` steps in one cycle, it SHALL stop the same way before the
    next draw or step.
  finding: 'The regenerative reading of max_draws (walk steps per test), and the TraceIndexRange exemption for the regenerative
    method in FR-189, have no acceptance criterion. FR-191-AC-2 tests max_draws only for the activation variant under Okamoto.
    Fix: add to FR-191-AC-2 (TC-626 step 2) a regenerative run with max_draws below the steps one cycle needs that stops Limit{MaxDraws,
    value}, and a regenerative request with m = 2 and max_draws = u64::MAX that is not refused TraceIndexRange.'
bindings: []
```

+++
