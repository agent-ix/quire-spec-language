<!-- reviewer-dispositions repo=agent-ix/quire-spec-language visibility=public quoin=0.28.3 module=spec-artifacts-process@b82e42e893612b4b4ee94127714b4577ee893502 id=SR-13547 round=1 model=claude-opus-5-5 run=9F8790A4-6653-4A89-BBBF-6C1FC5C1A890 -->

SR file: `reviews/qsl-570-pr1098-spec-review-integrity.md` (SR-13547), round 1 at head 7ff6d6491.

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | - |
| FND-002 | fixed | - |
| FND-003 | fixed | - |

New findings this round:

| FND | Severity | Check | Summary |
| --- | --- | --- | --- |
| FND-004 | medium | soundness | FR-192 settles every unnamed expansion stop as a broken invariant, though most are not internal failures, and FR-149 settles the same stop incomplete |
| FND-005 | low | other | FR-192 Inputs omits the request's FR-120 limits that the named-limit row now says the map receives |

Verdict: FND-001 to FND-003 are fixed: no resource-exhausted cause remains, the map again covers every unnamed expansion stop, and the named-limit row states the value's source and a setting name. New: FND-004 (medium), because every unnamed stop, internal or not, is settled as a broken invariant, unlike FR-149 and the model-check engine; FND-005 (low), because FR-192 Inputs omits the FR-120 limits. Not mergeable until FND-004 is fixed or the owner accepts it with a stated reason.

+++ [reviewer data]

```yaml
dispositions:
- fnd: FND-001
  outcome: fixed
  after_excerpt: '| `Stopped` by a sample expansion that names no limit, whether an internal failure or not | `failed` | diagnostic
    `runtime_invariant`/`established-invariant-broken` | none | `Failed` | failed (ADR-013 O-16 internal failure) |'
- fnd: FND-002
  outcome: fixed
  after_excerpt: '| `Stopped` by a sample expansion that names no limit, whether an internal failure or not | `failed` | diagnostic
    `runtime_invariant`/`established-invariant-broken` | none | `Failed` | failed (ADR-013 O-16 internal failure) |'
- fnd: FND-003
  outcome: fixed
  after_excerpt: '`limit-reached{limit, value, setting}`: `limit` the stop''s, `value` the configured bound of that limit
    in the request''s FR-120 limits (the evaluation meter''s budget, or `max_candidates`), which the map receives with the
    outcome because the stop carries only its cause and limit, and `setting` the name the request raises it by (`model_check.meter`,
    `model_check.max_candidates`)'
new_findings:
- fnd: FND-004
  severity: medium
  confidence: medium
  method: spec-review/integrity
  check: soundness
  artifact_id: FR-192-expansion-unnamed-row
  related_ids:
  - FR-149
  - PR-1079-code
  path: spec/functional/FR-192-settle-a-statistical-result-on-the-measured-axis.md
  lines: '101'
  excerpt: '| `Stopped` by a sample expansion that names no limit, whether an internal failure or not | `failed` | diagnostic
    `runtime_invariant`/`established-invariant-broken` | none | `Failed` | failed (ADR-013 O-16 internal failure) |'
  finding: 'The row now gives every expansion stop that names no limit the diagnostic runtime_invariant/established-invariant-broken,
    an O-16 internal failure. Most such stops are not internal failures. ExpansionStop::is_internal_failure separates them,
    and FR-149-AC-10''s example is a state-constraint family that evaluation refuses, an input cause. So the record calls
    a model''s own refusal a QSL defect. It also disagrees with QSL''s other engines. FR-149 (merged, #1097) and FR-144 settle
    that same non-internal stop Incomplete(ResourceExhausted) under V-7, and the model-check engine maps it to ResourceExhausted,
    not a fault (state_graph.rs:319-336). The PR body''s ''like the model-check engine'' is therefore wrong for the non-internal
    half. Fix (owner decision): keep runtime_invariant/established-invariant-broken for an is_internal_failure stop only.
    Give a non-internal unnamed stop a `failed` record whose diagnostic is the stop''s own catalog cause (FR-331''s `failed`
    carries exactly one diagnostic, and QSpec FR-408 says such a stop ''stops the run with that cause''), or settle it as
    FR-149 does once QSpec admits a matching incomplete cause. Correct the PR body either way.'
- fnd: FND-005
  severity: low
  confidence: high
  method: spec-review/integrity
  check: other
  artifact_id: FR-192-inputs
  related_ids:
  - FR-192-expansion-limit-row
  path: spec/functional/FR-192-settle-a-statistical-result-on-the-measured-axis.md
  lines: 46-47
  excerpt: A `StatisticalOutcome` (FR-189, FR-190) or a `StatisticalRefusal` (FR-189).
  finding: 'The named-limit row now says the map ''receives with the outcome'' the configured bound in the request''s FR-120
    limits, but FR-192 Inputs still lists only a StatisticalOutcome or a StatisticalRefusal. Fix: add the request''s FR-120
    limits (the evaluation meter budget and max_candidates) to Inputs, so the map''s signature can be read from the requirement.'
```

+++
