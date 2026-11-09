<!-- reviewer-dispositions repo=agent-ix/quire-spec-language visibility=public quoin=0.29.1 module=spec-artifacts-process@b82e42e893612b4b4ee94127714b4577ee893502 id=SR-971 round=1 model=claude-opus-5-5 run=063bcbf5-2612-42c6-86c5-0d8dcae5d1de -->

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | - |
| FND-002 | fixed | - |
| FND-003 | fixed | - |
| FND-004 | fixed | - |

New findings (disposition pass 1):

| FND | Severity | Check | Summary |
| --- | --- | --- | --- |
| FND-005 | medium | ambiguous | `field` is the S2 `Expression::Field` variant (qsl-forms/src/syntax.rs:663-668, `e.f`, operand "the record, or `deref(r)`"), the same form a Value-family record field read uses |
| FND-006 | low | soundness | The family column gives `reaches` as `ProtocolClause`; ADR-012 §15.2 assigns "Field reads, `deref`, `reaches` and object identity equality" to `StateModel` (the S2 variant lives in forms::protocol_clause per §15.3, which is where the confusion comes from) |
| FND-007 | medium | ambiguous | The fix dropped `forall`, `exists` and `present` and says they "fall under the rows of the table above by their owning form" because ADR-012 "names [them] as no form" |

+++ [reviewer data]

```yaml
dispositions:
- fnd: FND-001
  outcome: fixed
  after_excerpt: '- When a backend cannot translate a construct of a selected entry, `prepare` shall return `Ok` with that entry listed by `unsupported` with an `Untranslatable` cause, and shall prepare the other selected entries. - When the host environment refuses a resource every entry of a backend needs, `prepare` shall return `Ok` with every selected entry listed by `unsupported` with an `EnvironmentRefused` cause naming the refusal.'
- fnd: FND-002
  outcome: fixed
  after_excerpt: 'The v1 coverage lists no construct of the `StateModel` or `ProtocolClause` families and no `SumCase` construct. The first such construct of an entry is `ExpressionForm(name)`, `name` the construct''s ADR-012 name, for each of:  | Construct | `name` | Family (ADR-012) | | --- | --- | --- | | a dereference of a model element reference | `deref` | `StateModel` | | a field read on a model element | `field` | `StateModel` | | a reachability test over a state clause''s population | `reaches` | `ProtocolClause` | | the `self` of a state clause | `self` | `ProtocolClause` |'
- fnd: FND-003
  outcome: fixed
  after_excerpt: 'PendingEngine { verb: Verb, kind: OutcomeKind, reason: &''static str },'
- fnd: FND-004
  outcome: fixed
  after_excerpt: 'Each of the eight kinds has at least one applicable cell in `check`, `package`, `monitor` or `execute`, so every kind is evidenced without the pending cells: success by `check`,'
new_findings:
- fnd: FND-005
  severity: medium
  confidence: medium
  method: spec-review/spec-integrity-analysis
  check: ambiguous
  artifact_id: FR-560
  related_ids:
  - FR-560-AC-5
  - FR-560-AC-2
  - FR-294
  path: spec/functional/FR-560-translate-the-jit-coverage-set-and-report-every-other-construct-untranslatable.md
  lines: 69-78
  excerpt: 'The v1 coverage lists no construct of the `StateModel` or `ProtocolClause` families and no `SumCase` construct. The first such construct of an entry is `ExpressionForm(name)`, `name` the construct''s ADR-012 name, for each of:  | Construct | `name` | Family (ADR-012) | | --- | --- | --- | | a dereference of a model element reference | `deref` | `StateModel` | | a field read on a model element | `field` | `StateModel` | | a reachability test over a state clause''s population | `reaches` | `ProtocolClause` | | the `self` of a state clause | `self` | `ProtocolClause` |'
  finding: '`field` is the S2 `Expression::Field` variant (qsl-forms/src/syntax.rs:663-668, `e.f`, operand "the record, or `deref(r)`"), the same form a Value-family record field read uses. ADR-012 has no form-name catalogue, so naming it after the S2 variant is as legitimate as `deref`/`self`, but `ExpressionForm("field")` cannot tell a model field read from a record field read, and the floor''s form list (`literal` ... `collection`) does not say whether record field access lies in `record`. Either AC-5''s field-read entry or AC-2''s record units can be listed wrongly. State that a record field read lies in the `record` form and that only a field read whose operand is an object (model element) value is `ExpressionForm("field")`, or give that case `ValueType(<object type>)`.'
- fnd: FND-006
  severity: low
  confidence: high
  method: spec-review/spec-integrity-analysis
  check: soundness
  artifact_id: FR-560
  related_ids: []
  path: spec/functional/FR-560-translate-the-jit-coverage-set-and-report-every-other-construct-untranslatable.md
  lines: '77'
  excerpt: '| a reachability test over a state clause''s population | `reaches` | `ProtocolClause` |'
  finding: The family column gives `reaches` as `ProtocolClause`; ADR-012 §15.2 assigns "Field reads, `deref`, `reaches` and object identity equality" to `StateModel` (the S2 variant lives in forms::protocol_clause per §15.3, which is where the confusion comes from). Use `StateModel`, or drop the informational column.
- fnd: FND-007
  severity: medium
  confidence: high
  method: spec-review/spec-integrity-analysis
  check: ambiguous
  artifact_id: FR-560
  related_ids:
  - FR-560-AC-5
  path: spec/functional/FR-560-translate-the-jit-coverage-set-and-report-every-other-construct-untranslatable.md
  lines: 81-85
  excerpt: The quantifiers, the presence test and the other constructs that ADR-012 names as no form fall under the rows of the table above by their owning form, value type or operation, and have no row of their own. The interpreter, which FR-563 holds as the parity reference, is the only evaluator of such an entry; the JIT never runs it and `execute` never falls back (FR-296).
  finding: 'The fix dropped `forall`, `exists` and `present` and says they "fall under the rows of the table above by their owning form" because ADR-012 "names [them] as no form". That is not so: ADR-012:478 names the `Present` arm and keeps it in `Value`, and S2 has `Expression::Present` and the binder-query form with `BinderQuery::Forall`/`Exists` (qsl-forms/src/syntax.rs:415-426, 671). None of them is in the floor''s form list, so by the first row they are `ExpressionForm(<name>)`, but the spec no longer gives the name, and no AC covers them. Lead ruling (2) lists forall/exists/present as Untranslatable-with-named-cause in v1. Restore rows for them (for example `forall`, `exists` as the binder-query form''s names, `present`) and their AC-5 entries.'
```

+++
