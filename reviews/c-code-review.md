<!-- reviewer repo=agent-ix/quire-spec-language visibility=public quoin=0.29.1 module=spec-artifacts-process@b82e42e893612b4b4ee94127714b4577ee893502 id=SR-20261104153718 method=code-review lang=rust pr=quire-spec-language#2300 model=claude-opus-5-5 run=bd9f3f07-9d9e-4e1c-9fd2-53989df7c405 -->

| FND | Severity | Check | Summary |
| --- | --- | --- | --- |
| FND-001 | low | coverage | New AC-3 plants prove only the exemption; no new plant shows a node-bearing contract-model type beside an exempt one still fails |

Verdict: MERGEABLE. The five types at quire-contract-model 64de9ec hold identity or clock data only. The deny moves and the seam location follow the matches to their new homes. The hand-written applies_to_declaration is exhaustive and equal to the macro table. The TC-244 and TC-260 rows name real constructors and lookups.

+++ [reviewer data]

```yaml
clean: false
scope:
  - {id: "execute.rs:replay_with_cancel_and_temporal_selections", path: qsl-replay/src/execute.rs, role: examined, excerpt: "#[deny(clippy::wildcard_enum_match_arm)] pub fn replay_with_cancel_and_temporal_selections("}
  - {id: "value_check.rs:evaluate_with_case_budget", path: qsl-replay/src/value_check.rs, role: examined, excerpt: "#[deny(clippy::wildcard_enum_match_arm)] pub fn evaluate_with_case_budget("}
  - {id: "capability.rs:applies_to_declaration", path: qsl-semantics/src/check/capability.rs, role: examined, excerpt: "#[deny(clippy::wildcard_enum_match_arm)] pub fn applies_to_declaration(self, family: DeclarationFamily) -> bool"}
  - {id: "checked_input.rs:ADDRESSING_TYPES", path: xtask/src/checked_input.rs, role: examined, excerpt: "|| (first == CONTRACT_MODEL && !ADDRESSING_TYPES.contains(&last.as_str()))"}
  - {id: "seam_probe.rs:closed_enums_carry_no_non_exhaustive_attribute", path: xtask/src/seam_probe.rs, role: examined, excerpt: "(\"quire-checked\", \"node.rs\", \"NodeKind\"),"}
  - {id: "typestate_scan.rs:STAGE_CONSTRUCTORS", path: xtask/src/typestate_scan.rs, role: examined, excerpt: "const STAGE_CONSTRUCTORS: [(&str, Option<&str>, &str); 17]"}
  - {id: "typestate_scan.rs:RAW_WIRE_TYPE_FIELDS", path: xtask/src/typestate_scan.rs, role: examined, excerpt: "const RAW_WIRE_TYPE_FIELDS: [(&str, &str, &str); 6]"}
findings:
  - fnd: FND-001
    severity: low
    confidence: medium
    method: code-review
    check: coverage
    artifact_id: FR-270-AC-3
    related_ids: [FR-270-AC-1]
    path: xtask/src/checked_input.rs
    lines: "1184-1192"
    excerpt: |-
      (EVAL_PLANT, "use quire_contract_model::CheckedNodeId;\npub fn plant(c: &CheckedNodeId) {}\n"),
      (REPLAY_PLANT, "pub fn plant(i: &quire_contract_model::SourceIdentity) {}\n"),
    finding: "The two new AC-3 plants show only that the exemption admits CheckedNodeId and SourceIdentity. Only the unchanged AC-1 ContractPackage plant (full path) proves the rest of the set still fires. Add one AC-1 plant that imports an exempt type and ContractPackage in one use group and expects exactly one finding naming ContractPackage."
bindings:
  - {test_id: TC-745, ac_id: FR-270-AC-3, trace: correct}
  - {test_id: TC-161, ac_id: FR-063-AC-7, trace: correct}
```

+++
