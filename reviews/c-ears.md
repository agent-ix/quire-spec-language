<!-- reviewer repo=agent-ix/quire-spec-language visibility=public quoin=0.28.3 module=spec-artifacts-process@b82e42e893612b4b4ee94127714b4577ee893502 id=SR-13265 method=spec-review/ears lang=spec pr=quire-spec-language#1039 model=claude-opus-5-5 run=2A534AD9-7674-409A-9821-975EA1D37A8D -->

| FND | Severity | Check | Summary |
| --- | --- | --- | --- |
| FND-001 | low | other | The trigger sentence has EARS event shape but no SHALL, so the conditional typing binds only through sentence 1's reference to FR-149 |

+++ [reviewer data]

```yaml
clean: false
scope:
- id: FR-381
  path: spec/functional/FR-381-check-protocol-captures-activation-and-compensation-recovery-at-s3.md
  role: examined
  excerpt: 'S3 SHALL type an operand of `=` or `!=` in a retry relation or a recovery predicate by QSpec FR-149''s expected-type rule. When one operand has type `Int[lo,hi]` and the other, ignoring parentheses, is a binary `+`, `-` or `*` whose operands are `Integer` or `Int[..]`, the `Int[lo,hi]` operand supplies the arithmetic node''s expected type: the node is typed `Int[lo,hi]` with the obligation that its interval, under the facts that hold where it executes, lies inside `[lo,hi]`, a guard such as `earlier.tally < 3` supplying such a fact. S3 SHALL refuse an unproved obligation with `undefined_expressi'
findings:
- fnd: FND-001
  severity: low
  confidence: medium
  method: spec-review/ears
  check: other
  artifact_id: FR-381
  related_ids:
  - FR-149
  path: spec/functional/FR-381-check-protocol-captures-activation-and-compensation-recovery-at-s3.md
  lines: 224-230
  excerpt: 'When one operand has type `Int[lo,hi]` and the other, ignoring parentheses, is a binary `+`, `-` or `*` whose operands are `Integer` or `Int[..]`, the `Int[lo,hi]` operand supplies the arithmetic node''s expected type: the node is typed `Int[lo,hi]` with the obligation that its interval, under the facts that hold where it executes, lies inside `[lo,hi]`, a guard such as `earlier.tally < 3` supplying such a fact.'
  finding: 'The trigger sentence has EARS event shape but no SHALL, so the conditional typing binds only through sentence 1''s reference to FR-149. EARS form: ''When ..., S3 SHALL type the arithmetic node Int[lo,hi] with the obligation that ...''.'
bindings: []
```

+++
