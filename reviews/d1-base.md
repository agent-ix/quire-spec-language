<!-- reviewer-dispositions repo=agent-ix/quire-spec-language visibility=public quoin=0.28.3 module=spec-artifacts-process@b82e42e893612b4b4ee94127714b4577ee893502 id=SR-13264 round=1 model=claude-opus-5-5 run=2A534AD9-7674-409A-9821-975EA1D37A8D -->

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | - |
| FND-002 | fixed | - |
| FND-003 | fixed | - |
| FND-004 | fixed | - |

New findings (disposition pass 1):

| FND | Severity | Check | Summary |
| --- | --- | --- | --- |
| FND-005 | medium | soundness | AC-12 and TC-1084 step 3 require `earlier.tally + 1 <= later.tally` (an Integer sum against Int[0,3] under <=) to refuse ill_typed/type-mismatch, as QSpec FR-149-AC-21 says |
| FND-006 | low | ambiguous | The fixture gives 'parameters earlier and later' but checks the same text as a guard, an invariant and a function body |
| FND-007 | low | trace | FR-104 now restates QSpec FR-149's rule as normative but has no ix://agent-ix/quire-specification/FR-149 relationship in its frontmatter; add a depends_on edge. |

+++ [reviewer data]

```yaml
dispositions:
- fnd: FND-001
  outcome: fixed
  after_excerpt: '| 16a | the retry relation `later.tally = -earlier.tally` | `ill_typed`/`type-mismatch` at the comparison `later.tally = -earlier.tally` |'
- fnd: FND-002
  outcome: fixed
  after_excerpt: 'The checker SHALL NOT apply that rule to any other form: an `Integer`-typed unary `-`, path, call, `div`, `rem`, `count`, `sum` or unannotated `let`-bound name against an `Int[..]` operand, and an arithmetic operand against a `Rational[..]` or `Decimal[..]` operand, each refuse `ill_typed`/`type-mismatch` at the comparison, and `<`, `<=`, `>` and `>=` supply no expected type. An operand of the same `Int[..]` type as its peer is accepted unchanged.'
- fnd: FND-003
  outcome: fixed
  after_excerpt: 'The checker SHALL NOT apply that rule to any other form: an `Integer`-typed unary `-`, path, call, `div`, `rem`, `count`, `sum` or unannotated `let`-bound name against an `Int[..]` operand, and an arithmetic operand against a `Rational[..]` or `Decimal[..]` operand, each refuse `ill_typed`/`type-mismatch` at the comparison, and `<`, `<=`, `>` and `>=` supply no expected type. An operand of the same `Int[..]` type as its peer is accepted unchanged.'
- fnd: FND-004
  outcome: fixed
  after_excerpt: 'The checker SHALL type every `=` and `!=` in a clause by QSpec FR-149''s expected-type rule for an arithmetic operand against a bounded integer, wherever the equality is written: a guard, a capture, an invariant, a function body, a retry relation or a recovery predicate.'
new_findings:
- fnd: FND-005
  severity: medium
  confidence: high
  method: spec-review/base
  check: soundness
  artifact_id: FR-104-AC-12
  related_ids:
  - TC-1084
  - FR-149-AC-21
  path: spec/functional/FR-104-check-state-clauses.md
  lines: '309'
  excerpt: With `tally` of `Int[0,3]`, `earlier.tally < 3 and later.tally = earlier.tally + 1` checks in a guard, an invariant and a function body alike; with the guard removed each refuses `undefined_expression`/`unproved-range` at `earlier.tally + 1`. `later.tally = earlier.tally` checks; `later.tally = -earlier.tally`, `later.tally = n` for `n` of `Integer`, and `earlier.tally + 1 <= later.tally` each refuse `ill_typed`/`type-mismatch` at the comparison.
  finding: 'AC-12 and TC-1084 step 3 require `earlier.tally + 1 <= later.tally` (an Integer sum against Int[0,3] under <=) to refuse ill_typed/type-mismatch, as QSpec FR-149-AC-21 says. The QSL checker accepts it: ordering admits any Integer/Int(_) pair as OrderedKind::Integers (check.rs:1444, 2245), and the ordering peer path does no coercion (typing.rs:2048-2069). A test written to the row fails, and adding the refusal would reject every existing mixed Integer/Int[..] ordering. Needs a QSL code ticket, or a QSpec check that FR-149-AC-21 means to ban mixed integer ordering. Planner decision.'
- fnd: FND-006
  severity: low
  confidence: medium
  method: spec-review/base
  check: ambiguous
  artifact_id: TC-1084
  related_ids:
  - FR-104-AC-12
  - FR-104
  path: spec/test-cases/TC-1084-equality-types-an-arithmetic-operand-against-a-bounded-integer.md
  lines: 19-21
  excerpt: 'Fixture: a record with `tally: Int[0,3]`, parameters `earlier` and `later` of it, and `n` of `Integer`.

    1. Check `earlier.tally < 3 and later.tally = earlier.tally + 1` as a guard, as an invariant and as a function body; then each without the guard.'
  finding: The fixture gives 'parameters earlier and later' but checks the same text as a guard, an invariant and a function body. An invariant binds self, not parameters, and 'a guard' names no clause. FR-104's bullet also lists 'a function body' among positions 'in a clause'. Name the clause or binder for each position.
- fnd: FND-007
  severity: low
  confidence: high
  method: spec-review/base
  check: trace
  artifact_id: FR-104
  related_ids:
  - FR-149
  path: spec/functional/FR-104-check-state-clauses.md
  lines: 1-32
  excerpt: 'The checker SHALL type every `=` and `!=` in a clause by QSpec FR-149''s expected-type rule for an arithmetic operand against a bounded integer, wherever the equality is written: a guard, a capture, an invariant, a function body, a retry relation or a recovery predicate.'
  finding: FR-104 now restates QSpec FR-149's rule as normative but has no ix://agent-ix/quire-specification/FR-149 relationship in its frontmatter; add a depends_on edge.
```

+++
