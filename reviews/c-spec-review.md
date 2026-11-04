<!-- reviewer repo=agent-ix/quire-spec-language visibility=public quoin=0.29.1 module=spec-artifacts-process@b82e42e893612b4b4ee94127714b4577ee893502 id=SR-20261104153720 method=spec-review/integrity lang=spec pr=quire-spec-language#2300 model=claude-opus-5-5 run=bd9f3f07-9d9e-4e1c-9fd2-53989df7c405 -->

| FND | Severity | Check | Summary |
| --- | --- | --- | --- |
| FND-001 | low | ambiguous | FR-270 says FR-516/FR-522/FR-514 name the types; they name the shapes (occurrence {role, ordinal}, temporal_bindings row, formal identity), not the type names |
| FND-002 | low | coverage | FR-270-AC-1 has no case that a non-exempt contract-model type beside an exempt one still fails |

Verdict: MERGEABLE. The CK-2 (c) carve-out is a real refinement, not a lowered gate. It is bounded by a stated property (holds no checked node or term) and by a closed list of names. It resolves a conflict with FR-516, FR-522 and FR-514, and the AC-1 package plant still fails.

+++ [reviewer data]

```yaml
clean: false
scope:
  - {id: ADR-032, path: spec/decisions/ADR-032-checked-input-and-duplicate-canonical-type-gates.md, role: examined, excerpt: "(c) a type defined in quire-contract-model, which only the I2 reader in qsl-package::checked_v2 admits, except an addressing value: an identity key or clock row a stage entry selects with from a package the I2 reader already admitted, which holds no checked node or term and so cannot be executed"}
  - {id: FR-270, path: spec/functional/FR-270-gate-stage-entries-to-checked-inputs.md, role: examined, excerpt: "The gate SHALL treat as an addressing value, outside the set, a quire-contract-model identity key or clock row that a stage entry selects with from a package the I2 reader already admitted: CheckedNodeId and CheckedOccurrence (FR-516), TemporalBinding and ClockConfiguration (FR-522), and SourceIdentity (FR-514)."}
  - {id: FR-270-AC-1, path: spec/functional/FR-270-gate-stage-entries-to-checked-inputs.md, role: examined, excerpt: "and a pub fn in qsl-replay outside spine taking a quire-contract-model package, each make the gate exit non-zero with one signature finding"}
  - {id: FR-270-AC-3, path: spec/functional/FR-270-gate-stage-entries-to-checked-inputs.md, role: examined, excerpt: "a pub fn in qsl-eval taking quire_contract_model::CheckedNodeId, a pub fn in qsl-replay outside spine taking quire_contract_model::SourceIdentity ... give no finding."}
  - {id: FR-516, path: spec/functional/FR-516-carry-the-temporal-evidence-request-and-result-on-the-checked-package-route.md, role: context_only, excerpt: "control: Option<CheckedNodeId> ... occurrence: Option<{ role, ordinal }>"}
  - {id: FR-522, path: spec/functional/FR-522-bind-a-temporal-trace-to-the-checked-clause-s-profile-and-clock-and-report-the-authenticated-clock.md, role: context_only, excerpt: "the clock from the temporal_bindings row keyed by the declaration and the occurrence"}
  - {id: FR-514, path: spec/functional/FR-514-specify-the-spine-type-report-the-inventory-on-exhaustion-and-the-multi-unit-cycle-refusals.md, role: context_only, excerpt: "The selected formal identity SHALL be a parameter of qsl_inventory."}
findings:
  - fnd: FND-001
    severity: low
    confidence: high
    method: spec-review/integrity
    check: ambiguous
    artifact_id: FR-270
    related_ids: [ADR-032, FR-516, FR-522, FR-514]
    path: spec/functional/FR-270-gate-stage-entries-to-checked-inputs.md
    lines: "58-63"
    excerpt: |-
      CheckedNodeId and CheckedOccurrence (FR-516), TemporalBinding and ClockConfiguration (FR-522), and SourceIdentity (FR-514).
    finding: "FR-516 names the occurrence as { role, ordinal }, FR-522 names the temporal_bindings row, and FR-514 names the selected formal identity. None of them names the type itself. Reword the citations to the shapes they name, or cite the code item that carries each."
  - fnd: FND-002
    severity: low
    confidence: medium
    method: spec-review/integrity
    check: coverage
    artifact_id: FR-270-AC-1
    related_ids: [FR-270-AC-3]
    path: spec/functional/FR-270-gate-stage-entries-to-checked-inputs.md
    lines: "91"
    excerpt: |-
      and a pub fn in qsl-replay outside spine taking a quire-contract-model package, each make the gate exit non-zero with one signature finding naming the planted file and line, the entry and the type.
    finding: "No criterion pins the exact-name boundary of the carve-out. Add an AC-1 case where a non-exempt contract-model type sits beside an exempt one and still gives one finding."
bindings: []
```

+++
