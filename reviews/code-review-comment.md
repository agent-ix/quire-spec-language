<!-- reviewer repo=agent-ix/quire-spec-language visibility=public quoin=0.29.1 module=spec-artifacts-process@b82e42e893612b4b4ee94127714b4577ee893502 id=SR-202610310139 method=code-review lang=rust pr=quire-spec-language#2013 model=gpt-6.1-sol run=044983FD-5D00-418E-BF08-1DBA0E592802 -->

| FND | Severity | Check | Summary |
| --- | --- | --- | --- |
| FND-001 | medium | test-intent | The shared-key regression does not assert occurrence-specific conversion targets in serialized equality sidecar rows. |

+++ [reviewer data]

```yaml
clean: false
scope:
  - {id: QSpec-FR-322-AC-288, path: "QSpec/spec/objects/interfaces/FR-322-checked-package-artifact.md", role: examined, excerpt: "Each row retains the `equal` or `not_equal` operator, the two original operand positions in order, each operand's source type and producer-selected conversion target (or `null`), the selected `text`, `enum`, `quantity` or `plan` schedule, the resolved top-level quantity unit facts, and the filtered enum member facts."}
  - {id: IR-FR-353-AC-1, path: "quire-contract-ir/spec/checked_package/functional/FR-353-read-admitted-equality-decisions.md", role: context_only, excerpt: "A public external consumer reads admitted `=` and `!=` applications by each equality application's full node identity and exact occurrence and receives QSV `AdmittedEqualityDecision`; its read-only typed API exposes the producer-selected operator, original operand positions, source and conversion target types, and exact schedule in source order."}
  - {id: FR-093-AC-31, path: "spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md", role: context_only, excerpt: "A conversion between Rational, Decimal and Int types in either direction, and a Quantity conversion `convert<Km>(q) = r` (`quire.op.quantity.convert` with mode `rounding` = `exact`, then `quire.op.quantity.eq`), each lowers a convert node and the equality of the converted type."}
  - {id: FR-360-AC-8, path: "spec/functional/FR-360-type-a-field-by-a-variant-type-through-intake-check-evaluation-and-replay.md", role: context_only, excerpt: "The checked package emits `Config::Tier` as a `model`/`variant_type` node, the case literal as an `enum_value` of it and the argument by `ref` with no conversion node, and reads back `Verified`, also for a type whose cases include `Gold Plus`, `2x` and `Café`."}
  - {id: QSL-1390, path: "Linear QSL-1390", role: context_only, excerpt: "The emitter captures the equality schedule, conversion-target decisions, resolved UnitTable and filtered EnumMemberIndex at emit/admission time into the checked package; the reader consumes IR's typed no-recheck read and does not expose a public constructor from arbitrary data."}
  - {id: qsl-package/src/emit.rs, path: "qsl-package/src/emit.rs", role: examined, excerpt: "Builds checked_term_data equality decision rows from retained producer data and identity witnesses, refusing when exact joins or required identity context are missing."}
  - {id: qsl-package/src/emit/tests/converted_equality.rs, path: "qsl-package/src/emit/tests/converted_equality.rs", role: examined, excerpt: "Tests source occurrence retention, shared structural equality keys, serialized sidecar shape, operand order, schedules, quantity units, and typed refusal for missing or duplicate witnesses."}
  - {id: qsl-package/src/emit/tests/variant_types.rs, path: "qsl-package/src/emit/tests/variant_types.rs", role: examined, excerpt: "Tests exact selected model-enum member identities in sidecar rows and retains typed refusal from the pinned IR reader."}
  - {id: qsl-semantics/src/check/ir.rs, path: "qsl-semantics/src/check/ir.rs", role: examined, excerpt: "Retains equality operator, schedule, authored and compared operands, resolved units, filtered enum-member index, and exact producer identity witnesses."}
  - {id: qsl-semantics/src/check/lowering.rs, path: "qsl-semantics/src/check/lowering.rs", role: examined, excerpt: "Stores producer data and identities under `(application key, source location)` and constructs exact unit, dimension, text/plan type and enum-member identities."}
  - {id: qsl-semantics/src/check/refusal.rs, path: "qsl-semantics/src/check/refusal.rs", role: examined, excerpt: "Adds a typed MissingEnumMember keying refusal when a compared enum member has no unique exact source or admitted-model identity."}
findings:
  - fnd: FND-001
    severity: medium
    confidence: high
    method: code-review
    check: test-intent
    artifact_id: QSpec-FR-322-AC-288
    related_ids: []
    path: qsl-package/src/emit/tests/converted_equality.rs
    lines: "142-186, 275-337"
    excerpt: "Each row retains the `equal` or `not_equal` operator, the two original operand positions in order, each operand's source type and producer-selected conversion target (or `null`), the selected `text`, `enum`, `quantity` or `plan` schedule, the resolved top-level quantity unit facts, and the filtered enum member facts."
    finding: "The structurally reused equality regression checks two producer entries share a NodeKey but differ in Location and authored conversion target, then checks emitted identity rows only. It never asserts the two serialized checked_term_data equality_decisions or their distinct conversion_target values. The separate sidecar test uses distinct application keys, so a regression that assigns both occurrences the same conversion target would pass despite the contract requiring occurrence-specific nullable conversion targets. Assert each sidecar row's application/occurrence key and left conversion_target (null versus the exact emitted ms type)."
bindings: []
```

+++
