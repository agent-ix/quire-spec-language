<!-- reviewer repo=agent-ix/quire-spec-language visibility=public quoin=0.28.3 module=spec-artifacts-process@b82e42e893612b4b4ee94127714b4577ee893502 id=SR-13546 method=spec-review/ears lang=spec pr=quire-spec-language#1098 model=claude-opus-5-5 run=9F8790A4-6653-4A89-BBBF-6C1FC5C1A890 -->

SR file: `reviews/qsl-570-pr1098-spec-review-ears.md` (SR-13546), head a53c86bc4

| FND | Severity | Check | Summary |
| --- | --- | --- | --- |
| FND-001 | low | other | FR-189's new event-sample sentence is passive with no system subject and holds two SHALLs |

Verdict: One low finding. The FR-191 budget sentence keeps its EARS event form ('When a test has drawn ... it SHALL stop'). The FR-189 TraceIndexRange text is a definition list, not a SHALL statement. The FR-192 rows sit under the existing exhaustive-map SHALL. The quire grammar lint reports nothing in the three files.

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
findings:
- fnd: FND-001
  severity: low
  confidence: high
  method: spec-review/ears
  check: other
  artifact_id: FR-189-mean-of-fraction-bullet
  related_ids: []
  path: spec/functional/FR-189-decide-a-probabilistic-claim-by-statistical-model-checking.md
  lines: 152-155
  excerpt: EN-4 SHALL decide a mean of a fraction by the fixed-sample method over the sample mean of the fraction. A request
    naming the sequential method for it SHALL be refused as `NotStatistical`, as FR-408 states. A sampler that returns an
    event sample for a mean-of-fraction claim SHALL be read as the fraction 1 when the event holds and 0 when it does not,
    and EN-4 SHALL not refuse it.
  finding: 'The new sentence makes the sampler''s sample the subject of a passive ''SHALL be read'', so it names no system
    that does the reading. It then joins a second obligation with ''and EN-4 SHALL not refuse it'', which makes it two requirements
    in one sentence, and writes ''SHALL not'' where FR-189 elsewhere writes ''SHALL NOT''. Fix: ''If the sampler returns an
    event sample for a mean-of-fraction claim, then EN-4 SHALL count it as the fraction 1 when the event holds and 0 when
    it does not.'' The no-refusal clause then follows from the positive rule, and can be dropped or kept as its own sentence.'
bindings: []
```

+++
