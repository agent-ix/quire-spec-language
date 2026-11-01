<!-- reviewer repo=agent-ix/quire-spec-language visibility=public quoin=0.29.1 module=spec-artifacts-process@b82e42e893612b4b4ee94127714b4577ee893502 id=SR-202610310140 method=gap-analysis lang=rust pr=quire-spec-language#2013 model=gpt-6.1-sol run=044983FD-5D00-418E-BF08-1DBA0E592802 -->

| FND | Severity | Check | Summary |
| --- | --- | --- | --- |
| FND-001 | medium | coverage | The shared-key regression does not verify occurrence-specific conversion targets in serialized equality sidecar rows. |

+++ [reviewer data]

```yaml
clean: false
scope:
  - {id: QSpec-FR-322-AC-288, path: "QSpec/spec/objects/interfaces/FR-322-checked-package-artifact.md", role: examined, excerpt: "Each row retains the `equal` or `not_equal` operator, the two original operand positions in order, each operand's source type and producer-selected conversion target (or `null`), the selected `text`, `enum`, `quantity` or `plan` schedule, the resolved top-level quantity unit facts, and the filtered enum member facts."}
  - {id: IR-FR-353-AC-1, path: "quire-contract-ir/spec/checked_package/functional/FR-353-read-admitted-equality-decisions.md", role: context_only, excerpt: "A public external consumer reads admitted `=` and `!=` applications by each equality application's full node identity and exact occurrence and receives QSV `AdmittedEqualityDecision`; its read-only typed API exposes the producer-selected operator, original operand positions, source and conversion target types, and exact schedule in source order."}
  - {id: FR-093-AC-31, path: "spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md", role: context_only, excerpt: "A conversion between Rational, Decimal and Int types in either direction, and a Quantity conversion `convert<Km>(q) = r` (`quire.op.quantity.convert` with mode `rounding` = `exact`, then `quire.op.quantity.eq`), each lowers a convert node and the equality of the converted type."}
  - {id: FR-360-AC-8, path: "spec/functional/FR-360-type-a-field-by-a-variant-type-through-intake-check-evaluation-and-replay.md", role: context_only, excerpt: "The checked package emits `Config::Tier` as a `model`/`variant_type` node, the case literal as an `enum_value` of it and the argument by `ref` with no conversion node, and reads back `Verified`, also for a type whose cases include `Gold Plus`, `2x` and `Café`."}
  - {id: qsl-package/src/emit.rs, path: "qsl-package/src/emit.rs", role: examined, excerpt: "Joins retained equality facts to exact source occurrences and builds typed sidecar decisions, quantity unit rows and enum member rows."}
  - {id: qsl-package/src/emit/tests/converted_equality.rs, path: "qsl-package/src/emit/tests/converted_equality.rs", role: examined, excerpt: "Covers schedules, unit facts, sidecar shape and missing witness refusals; the shared-key test does not inspect the serialized equality decisions."}
  - {id: qsl-package/src/emit/tests/variant_types.rs, path: "qsl-package/src/emit/tests/variant_types.rs", role: examined, excerpt: "Checks model enum sidecar entries against exact emitted enum and member nodes, including non-identifier case names."}
  - {id: qsl-semantics/src/check/lowering.rs, path: "qsl-semantics/src/check/lowering.rs", role: examined, excerpt: "Retains equality producer data under `(NodeKey, Location)` and derives exact unit, dimension, text/plan type and enum-member identities during lowering."}
  - {id: QSL-1390, path: "Linear QSL-1390", role: context_only, excerpt: "This PR is only the QSL producer-side equality sidecar; IR's trusted reader consumes typed no-recheck data after the separate IR work."}
findings:
  - fnd: FND-001
    severity: medium
    confidence: high
    method: gap-analysis
    check: coverage
    artifact_id: QSpec-FR-322-AC-288
    related_ids: []
    path: qsl-package/src/emit/tests/converted_equality.rs
    lines: "142-186, 275-337"
    excerpt: "Each row retains the `equal` or `not_equal` operator, the two original operand positions in order, each operand's source type and producer-selected conversion target (or `null`), the selected `text`, `enum`, `quantity` or `plan` schedule, the resolved top-level quantity unit facts, and the filtered enum member facts."
    finding: "The structurally reused equality test proves two producer entries share a NodeKey but differ in Location and authored conversion target, then checks only emitted identity rows. It does not assert the two checked_term_data equality_decisions rows or their distinct conversion_target values. The generic sidecar test uses separate application keys, so it cannot catch loss or cross-join of occurrence-specific decisions for a shared key."
bindings: []
```

+++
