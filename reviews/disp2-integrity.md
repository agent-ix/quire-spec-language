<!-- reviewer-dispositions repo=agent-ix/quire-spec-language visibility=public quoin=0.28.3 module=spec-artifacts-process@b82e42e893612b4b4ee94127714b4577ee893502 id=SR-13547 round=2 model=claude-opus-5-5 run=9F8790A4-6653-4A89-BBBF-6C1FC5C1A890 -->

SR file: `reviews/qsl-570-pr1098-spec-review-integrity.md` (SR-13547), round 2 at head 0ed9ee0a4.

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-004 | fixed | - |
| FND-005 | fixed | - |

New findings this round:

| FND | Severity | Check | Summary |
| --- | --- | --- | --- |
| FND-006 | high | soundness | The non-internal unnamed stop's record claims the evaluation meter was reached when no limit was |
| FND-007 | low | ambiguous | FR-192-AC-1 does not pin the non-internal unnamed stop's cause, which TC-627 step 1 does |

Verdict: FND-004 and FND-005 are fixed: only internal-failure stops settle `failed`, other unnamed stops are incomplete with FR-149's internal value, and Inputs lists the FR-120 limits. New: FND-006 (high), because the non-internal row's wire record names the evaluation meter as reached when no limit was; FND-007 (low), because AC-1 and TC-627 step 1 disagree on how much they pin down. Not mergeable until FND-006 and base FND-004 are fixed.

+++ [reviewer data]

```yaml
dispositions:
- fnd: FND-004
  outcome: fixed
  after_excerpt: '| `Stopped` by a sample expansion that names no limit and is an internal failure (`ExpansionStop::is_internal_failure`)
    | `failed` | diagnostic `runtime_invariant`/`established-invariant-broken` | none | `Failed` | failed (ADR-013 O-16 internal
    failure) | / non-internal: `incomplete` ... `Incomplete(ResourceExhausted)` internally, as FR-149 and FR-144 settle it
    (ADR-018 V-7)'
- fnd: FND-005
  outcome: fixed
  after_excerpt: '- The request''s FR-120 limits (the evaluation meter budget and `max_candidates`), from which a `Stopped`
    by an expansion takes its value.'
new_findings:
- fnd: FND-006
  severity: high
  confidence: high
  method: spec-review/integrity
  check: soundness
  artifact_id: FR-192-expansion-unnamed-noninternal-row
  related_ids:
  - QSpec-FR-331-incomplete
  - QSpec-FR-461
  path: spec/functional/FR-192-settle-a-statistical-result-on-the-measured-axis.md
  lines: '104'
  excerpt: '| `Stopped` by a sample expansion that names no limit and is not an internal failure (for example a state-constraint
    family that evaluation refuses) | `incomplete` | `limit-reached{limit, value, setting}` naming the evaluation meter (`model_check.meter`)
    and its configured value, the closest cause FR-331 admits | none | `Incomplete(ResourceExhausted)` internally, as FR-149
    and FR-144 settle it (ADR-018 V-7) | incomplete |'
  finding: 'By definition this stop names no limit, yet its record writes limit-reached naming the evaluation meter and its
    configured value. QSpec FR-331 defines limit-reached as ''naming the limit, its value and the request setting that raises
    it'', and FR-461-AC-6 expects raising that setting to let the same item finish. A caller who raises model_check.meter
    after a refused state-constraint family gets the same stop again. So the record misstates why the run stopped. The row
    also disagrees with itself: its TerminalValue Incomplete(ResourceExhausted) carries no limit, value or setting, so the
    map has no source for the wire record''s limit. Fix (owner decision): write a record FR-331 admits honestly, for example
    `failed` with the stop''s own catalog cause as its one diagnostic (FR-331''s failed admits a diagnostic; FR-408 says such
    a stop ''stops the run with that cause''), until the paired QSpec change adds an unnamed resource-exhaustion cause. Do
    not name a limit the run did not reach.'
- fnd: FND-007
  severity: low
  confidence: high
  method: spec-review/integrity
  check: ambiguous
  artifact_id: FR-192-AC-1
  related_ids:
  - FR-192-expansion-unnamed-noninternal-row
  path: spec/functional/FR-192-settle-a-statistical-result-on-the-measured-axis.md
  lines: '134'
  excerpt: '... one by an internal-failure expansion naming no limit settling `failed`, `runtime_invariant`/`established-invariant-broken`,
    and one by a non-internal expansion naming no limit settling `incomplete`. ...'
  finding: 'FR-192-AC-1 says only that the non-internal unnamed stop settles `incomplete`, while TC-627 step 1 expects `limit-reached`
    naming the evaluation meter. The AC does not check the cause, and the TC asserts something the AC does not state. Fix:
    once FND-006 is decided, state the record''s cause in the AC and keep TC-627 step 1 equal to it.'
```

+++
