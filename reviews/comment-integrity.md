<!-- reviewer repo=agent-ix/quire-spec-language visibility=public quoin=0.28.3 module=spec-artifacts-process@b82e42e893612b4b4ee94127714b4577ee893502 id=SR-13547 method=spec-review/integrity lang=spec pr=quire-spec-language#1098 model=claude-opus-5-5 run=9F8790A4-6653-4A89-BBBF-6C1FC5C1A890 -->

SR file: `reviews/qsl-570-pr1098-spec-review-integrity.md` (SR-13547), head a53c86bc4

| FND | Severity | Check | Summary |
| --- | --- | --- | --- |
| FND-001 | high | soundness | FR-192's unnamed expansion-stop row writes the cause `resource-exhausted`, which QSpec FR-331 does not admit |
| FND-002 | high | soundness | FR-192's map is no longer exhaustive: an internal-failure expansion stop has no row |
| FND-003 | medium | ambiguous | The named-limit expansion row's value and setting have no source in the map's inputs, and FR-126 names no setting |

Verdict: Two high findings and one medium. Not mergeable as written. The unnamed-stop row writes a wire cause that QSpec refuses. The map lost its exhaustiveness for internal-failure expansion stops. The named-limit row's value and setting have no stated source. What holds: the named-limit row's `limit-reached{limit, value, setting}` matches QSpec FR-408-AC-8 (the clause meter settles limit-reached) and FR-331. The FR-189 and FR-191 edits agree with FR-190 and with the code. Owner decision (event as 0/1): consistent with QSpec FR-408, whose mean-of-fraction values lie in [0, 1], and with the code. Owner decision (resource-exhausted): blocked by FND-001 until QSpec admits the spelling or the row uses one it already admits. No relationship edge is missing: FR-192 already depends_on FR-189.

+++ [reviewer data]

```yaml
clean: false
scope:
- id: FR-189-refusals
  path: spec/functional/FR-189-decide-a-probabilistic-claim-by-statistical-model-checking.md
  role: examined
  excerpt: '`TraceIndexRange` when `max_draws · m` overflows the trace index of Okamoto or SPRT (the regenerative test `j`
    uses trace index `j` alone, so the method never raises it), `ConfidenceBeyondPrecision` when `min(α'', β'')` has no normal-quantile
    bound for the regenerative method, `FractionForEvent` for a fraction a Bernoulli test cannot take'
- id: FR-189-mean-of-fraction-bullet
  path: spec/functional/FR-189-decide-a-probabilistic-claim-by-statistical-model-checking.md
  role: examined
  excerpt: EN-4 SHALL decide a mean of a fraction by the fixed-sample method over the sample mean of the fraction. A request
    naming the sequential method for it SHALL be refused as `NotStatistical`, as FR-408 states. A sampler that returns an
    event sample for a mean-of-fraction claim SHALL be read as the fraction 1 when the event holds and 0 when it does not,
    and EN-4 SHALL not refuse it.
- id: FR-191-budget-bullet
  path: spec/functional/FR-191-bound-and-reproduce-a-statistical-run.md
  role: examined
  excerpt: When a test has drawn `max_draws` samples (for the regenerative method, taken `max_draws` walk steps in total,
    as FR-190 counts steps as draws), or taken `max_cycle_steps` steps in one cycle, it SHALL stop the same way before the
    next draw or step.
- id: FR-192-map-exhaustive
  path: spec/functional/FR-192-settle-a-statistical-result-on-the-measured-axis.md
  role: examined
  excerpt: 'The map from `StatisticalOutcome` to `TerminalValue` SHALL be exhaustive, with no `_` arm:'
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
- id: FR-192-inputs
  path: spec/functional/FR-192-settle-a-statistical-result-on-the-measured-axis.md
  role: examined
  excerpt: A `StatisticalOutcome` (FR-189, FR-190) or a `StatisticalRefusal` (FR-189).
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
- id: QSpec-FR-331-incomplete
  path: agent-ix/quire-specification@2aadb7e spec/objects/interfaces/FR-331-backend-provider-envelope.md
  role: context_only
  excerpt: '| `incomplete` | `cause` (`limit-reached`, `precision-budget`, `cancelled{source}` or `timed-out`) |'
findings:
- fnd: FND-001
  severity: high
  confidence: high
  method: spec-review/integrity
  check: soundness
  artifact_id: FR-192-expansion-unnamed-row
  related_ids:
  - QSpec-FR-331-incomplete
  path: spec/functional/FR-192-settle-a-statistical-result-on-the-measured-axis.md
  lines: '101'
  excerpt: '| `Stopped` by a sample expansion that names no limit and is not an internal failure | `incomplete` | `resource-exhausted`
    | none | `Incomplete(ResourceExhausted)` (ADR-018 V-7) | incomplete |'
  finding: 'The row writes the cause `resource-exhausted`, which QSpec does not define. FR-331''s closed incomplete causes
    are limit-reached, timed-out, precision-budget and cancelled{source}. FR-331-AC-16 refuses any other spelling on a probabilistic
    result with invalid_result/unknown-cause. QSpec FR-408 says every statistical stop settles limit-reached or cancelled.
    QSL''s own internal Incomplete(ResourceExhausted) (FR-149, FR-358) is fine, but this column is the wire cause, so a reader
    would refuse this record. The same check was raised on PR #1082 (SR-13394 FND-004) for the unsupported causes. Fix (owner
    decision): give this stop an FR-331 spelling QSpec already admits, or pair this PR with a QSpec FR-331/FR-408 change that
    adds the cause, and name that change in References.'
- fnd: FND-002
  severity: high
  confidence: high
  method: spec-review/integrity
  check: soundness
  artifact_id: FR-192-map-exhaustive
  related_ids:
  - FR-192-expansion-unnamed-row
  - PR-1079-code
  path: spec/functional/FR-192-settle-a-statistical-result-on-the-measured-axis.md
  lines: 90-101
  excerpt: 'The map from `StatisticalOutcome` to `TerminalValue` SHALL be exhaustive, with no `_` arm:'
  finding: 'The map must be exhaustive with no `_` arm, but no row now covers a sample expansion that names no limit and is
    an internal failure. The new rows exclude it on purpose. FR-189 says every ExpansionStop returns Stopped with StatisticalStop::Expansion,
    and #1079''s halted() does exactly that, internal failures included (statistical.rs:282-293). So that outcome reaches
    the map with no row. The model-check engine refuses such a stop as a fault (state_graph.rs:319-336), and FR-149 calls
    an internal failure ''a fault of the checker''. Fix: add a row settling an internal-failure expansion stop `failed`, diagnostic
    runtime_invariant/established-invariant-broken, as the Internal refusal row does. Otherwise have FR-189 refuse it as Internal
    before it becomes a Stopped (that is a code change on #1079).'
- fnd: FND-003
  severity: medium
  confidence: high
  method: spec-review/integrity
  check: ambiguous
  artifact_id: FR-192-expansion-limit-row
  related_ids:
  - FR-192-inputs
  - PR-1079-code
  path: spec/functional/FR-192-settle-a-statistical-result-on-the-measured-axis.md
  lines: 46, 100
  excerpt: '| `Stopped` by a sample expansion (`Expansion(ExpansionStop)`) that names a limit (`EvaluationMeter` or `MaxCandidates`,
    FR-126) | `incomplete` | `limit-reached{limit, value, setting}` with the stop''s limit, the value it was set to and the
    request setting that raises it (FR-126''s setting for that limit) | none | `Incomplete(LimitReached{limit, value, setting})`
    (ADR-018 V-7) | incomplete |'
  finding: 'The named-limit row needs a value and a setting that its inputs do not hold. The map''s input is the StatisticalOutcome
    alone (FR-192 Inputs). Its Expansion(ExpansionStop) carries only cause and limit (expansion.rs:24-31). StatisticalRequest''s
    limits hold only max_samples, max_draws and max_cycle_steps. ''FR-126''s setting for that limit'' points at nothing either:
    FR-126 says only that EvaluationMeter and MaxCandidates are members of the subject''s FR-120 limits. FR-255''s table has
    no setting name for them, and QSpec FR-461 requires one dotted engine.member name per limit. Fix: say where the value
    comes from (the subject''s FR-120 limits passed to the map, or carried in the stop), and name the setting string for each
    of the two limits.'
bindings: []
```

+++
