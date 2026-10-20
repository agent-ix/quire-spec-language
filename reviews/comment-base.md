<!-- reviewer repo=agent-ix/quire-spec-language visibility=public quoin=0.28.3 module=spec-artifacts-process@b82e42e893612b4b4ee94127714b4577ee893502 id=SR-13570 method=spec-review/base lang=spec pr=quire-spec-language#1101 model=claude-opus-5-5 run=C6320F6A-0E94-4727-A3A0-A0E2D0A353B8 -->

SR file: `reviews/qsl-570-pr1101-spec-review-base.md` (SR-13570), head f810277ee

| FND | Severity | Check | Summary |
| --- | --- | --- | --- |
| FND-001 | low | other | TC-627 step 1 now repeats 'including' and reads as a run-on list |

Verdict: One low finding. FR-192-AC-1 and TC-627 step 1 each name the new row's result and cause, one for one, and TC-627's scope is unchanged (FR-192-AC-1 to AC-4).

+++ [reviewer data]

```yaml
clean: false
scope:
- id: FR-192-AC-1
  path: spec/functional/FR-192-settle-a-statistical-result-on-the-measured-axis.md
  role: examined
  excerpt: '... a `NoTest` refusal settling `inconclusive`, `no-initial-state`; a `ConfidenceBeyondPrecision` refusal settling
    `unsupported`, `confidence-beyond-precision`; an `Internal` refusal settling `failed`; ...'
- id: TC-627-step-1
  path: spec/test-cases/TC-627-statistical-results-settle-as-measured.md
  role: examined
  excerpt: '1. Map each outcome row, including a `ConfidenceBeyondPrecision` refusal, including a `Stopped` by an expansion
    naming `EvaluationMeter` and one naming no limit that is an internal failure, and one naming no limit that is not. / Step
    1: ... the `ConfidenceBeyondPrecision` refusal settles `unsupported`, `confidence-beyond-precision`; ...'
- id: FR-192-beyond-precision-row
  path: spec/functional/FR-192-settle-a-statistical-result-on-the-measured-axis.md
  role: examined
  excerpt: '| `ConfidenceBeyondPrecision` refusal | `unsupported` | the cause `confidence-beyond-precision` (QSpec FR-331,
    FR-408 Admission) | none | `Unsupported(cause)` | unsupported |'
- id: FR-189-refusals
  path: spec/functional/FR-189-decide-a-probabilistic-claim-by-statistical-model-checking.md
  role: context_only
  excerpt: '`ConfidenceBeyondPrecision` when `min(α'', β'')` has no normal-quantile bound for the regenerative method'
- id: PR-1079-code
  path: agent-ix/quire-spec-language@dd3a654d7 qsl-analyze/src/statistical.rs
  role: context_only
  excerpt: QuantileFailure::BeyondPrecision => StatisticalRefusal::ConfidenceBeyondPrecision (before any sample)
findings:
- fnd: FND-001
  severity: low
  confidence: high
  method: spec-review/base
  check: other
  artifact_id: TC-627-step-1
  related_ids:
  - FR-192-AC-1
  path: spec/test-cases/TC-627-statistical-results-settle-as-measured.md
  lines: '21'
  excerpt: '1. Map each outcome row, including a `ConfidenceBeyondPrecision` refusal, including a `Stopped` by an expansion
    naming `EvaluationMeter` and one naming no limit that is an internal failure, and one naming no limit that is not. / Step
    1: ... the `ConfidenceBeyondPrecision` refusal settles `unsupported`, `confidence-beyond-precision`; ...'
  finding: 'Step 1 now reads ''Map each outcome row, including a `ConfidenceBeyondPrecision` refusal, including a `Stopped`
    by an expansion naming `EvaluationMeter` and one naming no limit that is an internal failure, and one naming no limit
    that is not.'' The second ''including'' makes it unclear whether the expansion cases belong to the refusal. Fix: ''Map
    each outcome row, including a `ConfidenceBeyondPrecision` refusal, a `Stopped` by an expansion naming `EvaluationMeter`,
    and two expansion stops naming no limit, one an internal failure and one not.'''
bindings: []
```

+++
