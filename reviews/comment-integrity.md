<!-- reviewer repo=agent-ix/quire-spec-language visibility=public quoin=0.28.3 module=spec-artifacts-process@b82e42e893612b4b4ee94127714b4577ee893502 id=SR-13572 method=spec-review/integrity lang=spec pr=quire-spec-language#1101 model=claude-opus-5-5 run=C6320F6A-0E94-4727-A3A0-A0E2D0A353B8 -->

SR file: `reviews/qsl-570-pr1101-spec-review-integrity.md` (SR-13572), head f810277ee

| FND | Severity | Check | Summary |
| --- | --- | --- | --- |
| FND-001 | medium | soundness | FR-192's UnsupportedCause variant list omits ConfidenceBeyondPrecision, which the new row's Unsupported(cause) needs |

Verdict: One medium finding. What holds: the spelling `confidence-beyond-precision` matches QSpec #300's FR-408 Admission, FR-331 list, schema enum and vectors. The engine raises the refusal before any sample, as FR-408 Admission requires. The remaining four refusals keep the merged #1098 diagnostic row unchanged. Merge order: correct as the PR body states, after QSpec #300; #300 is still open, so #1101 is not mergeable yet even once FND-001 is fixed.

+++ [reviewer data]

```yaml
clean: false
scope:
- id: FR-192-beyond-precision-row
  path: spec/functional/FR-192-settle-a-statistical-result-on-the-measured-axis.md
  role: examined
  excerpt: '| `ConfidenceBeyondPrecision` refusal | `unsupported` | the cause `confidence-beyond-precision` (QSpec FR-331,
    FR-408 Admission) | none | `Unsupported(cause)` | unsupported |'
- id: FR-192-diagnostic-refusal-row
  path: spec/functional/FR-192-settle-a-statistical-result-on-the-measured-axis.md
  role: examined
  excerpt: '| `GeneratorMismatch`, `TooManyTests`, `TraceIndexRange` or `FractionForEvent` refusal | `unsupported` | diagnostic
    `unsupported_projection`/`unsupported-requested-capability` naming the refusal | none | `Unsupported(diagnostic)` | unsupported
    |'
- id: FR-192-outputs-unsupported-cause
  path: spec/functional/FR-192-settle-a-statistical-result-on-the-measured-axis.md
  role: examined
  excerpt: '// UnsupportedCause gains: NotMarkov{…}, TotalAboveDrawRange{total}, EveryScheduler, MissingConfidence, ExactOnlyForm'
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
- id: FR-189-refusals
  path: spec/functional/FR-189-decide-a-probabilistic-claim-by-statistical-model-checking.md
  role: examined
  excerpt: '`ConfidenceBeyondPrecision` when `min(α'', β'')` has no normal-quantile bound for the regenerative method'
- id: PR-1079-code
  path: agent-ix/quire-spec-language@dd3a654d7 qsl-analyze/src/statistical.rs
  role: context_only
  excerpt: QuantileFailure::BeyondPrecision => StatisticalRefusal::ConfidenceBeyondPrecision (before any sample)
- id: QSpec-PR-300
  path: agent-ix/quire-specification@1d1d3e976 FR-331, FR-408, FR-290, proposals/backend-provider-v1/schema.json, vectors.json
  role: context_only
  excerpt: '| `unsupported` (cause) | `missing-confidence`, `every-scheduler`, `exact-only-form`, `confidence-beyond-precision`,
    `not-markov{state, transition, post_states}`, ...'
findings:
- fnd: FND-001
  severity: medium
  confidence: high
  method: spec-review/integrity
  check: soundness
  artifact_id: FR-192-outputs-unsupported-cause
  related_ids:
  - FR-192-beyond-precision-row
  path: spec/functional/FR-192-settle-a-statistical-result-on-the-measured-axis.md
  lines: '73'
  excerpt: '// UnsupportedCause gains: NotMarkov{…}, TotalAboveDrawRange{total}, EveryScheduler, MissingConfidence, ExactOnlyForm'
  finding: 'The new row settles the refusal as `Unsupported(cause)` with cause `confidence-beyond-precision`, but the Outputs
    line that lists the variants `UnsupportedCause` gains still reads ''NotMarkov{…}, TotalAboveDrawRange{total}, EveryScheduler,
    MissingConfidence, ExactOnlyForm''. As written, the terminal value has no variant to carry the new cause, so an implementer
    following Outputs cannot build the row. Until this PR the refusal used the diagnostic form and needed no variant. Fix:
    add `ConfidenceBeyondPrecision` to the ''UnsupportedCause gains'' line, written as `confidence-beyond-precision` on the
    wire.'
bindings: []
```

+++
