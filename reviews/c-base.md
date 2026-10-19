<!-- reviewer repo=agent-ix/quire-spec-language visibility=public quoin=0.28.3 module=spec-artifacts-process@b82e42e893612b4b4ee94127714b4577ee893502 id=SR-13264 method=spec-review/base lang=spec pr=quire-spec-language#1039 model=claude-opus-5-5 run=2A534AD9-7674-409A-9821-975EA1D37A8D -->

| FND | Severity | Check | Summary |
| --- | --- | --- | --- |
| FND-001 | medium | soundness | Row 16a locates the ill_typed/type-mismatch for `later.tally = -earlier.tally` at `-earlier.tally` |
| FND-002 | medium | soundness | The no-coercion list drops FR-149's 'Integer' qualifier ('any other Integer operand'; AC-21 'an Integer path') |
| FND-003 | medium | ambiguous | The restatement reads as a closed list but is narrower than FR-149: it omits `sum`, omits that an arithmetic operand against Rational[..] or Decimal[..] stays ill_typed, and names only `<` where FR-149 says <, <=, > and >= supply no expected type |
| FND-004 | medium | coverage | The rule is scoped to '=' or '!=' in a retry relation or a recovery predicate, but FR-149 applies it to every =/!= and the QSL checker applies it on the one general equality path (admit_sum_into_peer, typing.rs:2099,2113), so guards, captures, invariants and function bodies get it too while QSL states it nowhere for them |

+++ [reviewer data]

```yaml
clean: false
scope:
- id: FR-381
  path: spec/functional/FR-381-check-protocol-captures-activation-and-compensation-recovery-at-s3.md
  role: examined
  excerpt: 'S3 SHALL type an operand of `=` or `!=` in a retry relation or a recovery predicate by QSpec FR-149''s expected-type rule. When one operand has type `Int[lo,hi]` and the other, ignoring parentheses, is a binary `+`, `-` or `*` whose operands are `Integer` or `Int[..]`, the `Int[lo,hi]` operand supplies the arithmetic node''s expected type: the node is typed `Int[lo,hi]` with the obligation that its interval, under the facts that hold where it executes, lies inside `[lo,hi]`, a guard such as `earlier.tally < 3` supplying such a fact. S3 SHALL refuse an unproved obligation with `undefined_expressi'
- id: FR-381-AC-10
  path: spec/functional/FR-381-check-protocol-captures-activation-and-compensation-recovery-at-s3.md
  role: examined
  excerpt: 'A retry relation `earlier.tally < 3 and later.tally = earlier.tally + 1` checks: the guard puts the sum''s interval `[1,3]` inside `Int[0,3]`, so the expected-type rule types the sum `Int[0,3]`. With `earlier.tally < 3 and` removed it refuses `undefined_expression`/`unproved-range` at `earlier.tally + 1`, the sum, as does the recovery predicate `recovered.tally = recovered.tally + 1` at its sum. `later.tally = -earlier.tally` refuses `ill_typed`/`type-mismatch`. A non-Boolean relation refuses `ill_typed`/`non-boolean-root`; a relation that reads a registration capture and an activation capture '
- id: TC-1058-row-16
  path: spec/test-cases/TC-1058-s3-checks-compensation-registration-retry-commit-and-recovery.md
  role: examined
  excerpt: '| 16 | the retry relation without `earlier.tally < 3 and` | `undefined_expression`/`unproved-range` at `earlier.tally + 1` |'
- id: TC-1058-row-16a
  path: spec/test-cases/TC-1058-s3-checks-compensation-registration-retry-commit-and-recovery.md
  role: examined
  excerpt: '| 16a | the retry relation `later.tally = -earlier.tally` | `ill_typed`/`type-mismatch` at `-earlier.tally` |'
- id: TC-1058-row-22
  path: spec/test-cases/TC-1058-s3-checks-compensation-registration-retry-commit-and-recovery.md
  role: examined
  excerpt: '| 22 | the recovery predicate `recovered.tally = recovered.tally + 1` | `undefined_expression`/`unproved-range` at `recovered.tally + 1` |'
- id: FR-149
  path: quire-specification:spec/functional/type-model/FR-149-apply-complete-equality-matrix.md
  role: context_only
  excerpt: 'The rule is narrow. Only a binary `+`, `-` or `*` node that is directly an operand of `=` or `!=` is typed this way. Unary `-` (typed `Integer`), a path, a call, `div`, `rem`, `count`, `sum`, a `let`-bound name without its own annotation and any other `Integer` operand against an `Int[..]` operand remain `refused { code: ill_typed, cause: type-mismatch }` without an explicit `convert`, as does an arithmetic operand against a `Rational[..]` or `Decimal[..]` operand. ... `<`, `<=`, `>` and `>=` supply no expected type to their operands.'
findings:
- fnd: FND-001
  severity: medium
  confidence: high
  method: spec-review/base
  check: soundness
  artifact_id: TC-1058-row-16a
  related_ids:
  - FR-381-AC-10
  - FR-149-AC-21
  - FR-056-AC-36
  - FR-056-AC-41
  path: spec/test-cases/TC-1058-s3-checks-compensation-registration-retry-commit-and-recovery.md
  lines: '53'
  excerpt: '| 16a | the retry relation `later.tally = -earlier.tally` | `ill_typed`/`type-mismatch` at `-earlier.tally` |'
  finding: 'Row 16a locates the ill_typed/type-mismatch for `later.tally = -earlier.tally` at `-earlier.tally`. QSpec FR-149-AC-21 gives no locus, so QSL''s convention governs: FR-056-AC-36/41 refuse an equality operand-type mismatch ''at the comparison'', and the checker does the same (unary - types Integer, check.rs:2431; admit_sum_into_peer leaves a non-arithmetic operand alone and equality maps the matrix refusal to the equality''s own location, check.rs:2223). Fix: ''at the comparison `later.tally = -earlier.tally`'', and name that locus in FR-381-AC-10.'
- fnd: FND-002
  severity: medium
  confidence: high
  method: spec-review/base
  check: soundness
  artifact_id: FR-381
  related_ids:
  - FR-149
  - TC-1058-row-15
  path: spec/functional/FR-381-check-protocol-captures-activation-and-compensation-recovery-at-s3.md
  lines: 232-235
  excerpt: 'The rule adds no conversion: a unary `-`, a path, a call, `div`, `rem`, `count` or an unannotated `let`-bound name against an `Int[..]` operand is `ill_typed`/`type-mismatch`, and `<` supplies no expected type.'
  finding: 'The no-coercion list drops FR-149''s ''Integer'' qualifier (''any other Integer operand''; AC-21 ''an Integer path''). As written an Int[0,3] path or call against Int[0,3] (`later.tally = earlier.tally`, the identical-type row) is refused, which FR-149 and the checker admit. Fix: ''an Integer-typed unary -, path, call, ...''.'
- fnd: FND-003
  severity: medium
  confidence: medium
  method: spec-review/base
  check: ambiguous
  artifact_id: FR-381
  related_ids:
  - FR-149
  - FR-149-AC-21
  path: spec/functional/FR-381-check-protocol-captures-activation-and-compensation-recovery-at-s3.md
  lines: 232-235
  excerpt: 'The rule adds no conversion: a unary `-`, a path, a call, `div`, `rem`, `count` or an unannotated `let`-bound name against an `Int[..]` operand is `ill_typed`/`type-mismatch`, and `<` supplies no expected type.'
  finding: 'The restatement reads as a closed list but is narrower than FR-149: it omits `sum`, omits that an arithmetic operand against Rational[..] or Decimal[..] stays ill_typed, and names only `<` where FR-149 says <, <=, > and >= supply no expected type. Fix: complete the list or defer wholly to FR-149.'
- fnd: FND-004
  severity: medium
  confidence: medium
  method: spec-review/base
  check: coverage
  artifact_id: FR-381
  related_ids:
  - FR-149
  - FR-104
  path: spec/functional/FR-381-check-protocol-captures-activation-and-compensation-recovery-at-s3.md
  lines: 223-224
  excerpt: 'S3 SHALL type an operand of `=` or `!=` in a retry relation or a recovery predicate by QSpec FR-149''s expected-type rule. When one operand has type `Int[lo,hi]` and the other, ignoring parentheses, is a binary `+`, `-` or `*` whose operands are `Integer` or `Int[..]`, the `Int[lo,hi]` operand supplies the arithmetic node''s expected type: the node is typed `Int[lo,hi]` with the obligation that its interval, under the facts that hold where it executes, lies inside `[lo,hi]`, a guard such as `earlier.tally < 3` supplying such a fact. S3 SHALL refuse an unproved obligation with `undefined_expressi'
  finding: 'The rule is scoped to ''='' or ''!='' in a retry relation or a recovery predicate, but FR-149 applies it to every =/!= and the QSL checker applies it on the one general equality path (admit_sum_into_peer, typing.rs:2099,2113), so guards, captures, invariants and function bodies get it too while QSL states it nowhere for them. Fix: state it once at the general typer (e.g. FR-104) and reference it from FR-381.'
bindings: []
```

+++
