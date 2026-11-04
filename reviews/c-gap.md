<!-- reviewer repo=agent-ix/quire-spec-language visibility=public quoin=0.29.1 module=spec-artifacts-process@b82e42e893612b4b4ee94127714b4577ee893502 id=SR-20261104153719 method=gap-analysis lang=rust pr=quire-spec-language#2300 model=claude-opus-5-5 run=bd9f3f07-9d9e-4e1c-9fd2-53989df7c405 -->

| FND | Severity | Check | Summary |
| --- | --- | --- | --- |
| FND-001 | low | coverage | No failing test pins the boundary between exempt and non-exempt names in the FR-270 carve-out |

Verdict: MERGEABLE. Every changed behaviour has a tagged test, and no test or check was deleted. Plan completion: not assessed.

+++ [reviewer data]

```yaml
clean: false
scope:
  - {id: FR-270-AC-3, path: xtask/src/checked_input.rs, role: examined, excerpt: "ADR-032 CK-2 (c): an addressing value into an admitted package."}
  - {id: FR-063-AC-7, path: xtask/src/seam_probe.rs, role: examined, excerpt: "closed_enums_carry_no_non_exhaustive_attribute"}
  - {id: FR-507, path: qsl-semantics/src/check/capability.rs, role: examined, excerpt: "unexpected FR-507 applicability for {capability:?} on {family:?}"}
  - {id: TC-244, path: xtask/src/typestate_scan.rs, role: examined, excerpt: "STAGE_CONSTRUCTORS"}
  - {id: TC-260, path: xtask/src/typestate_scan.rs, role: examined, excerpt: "RAW_WIRE_TYPE_FIELDS"}
findings:
  - fnd: FND-001
    severity: low
    confidence: medium
    method: gap-analysis
    check: coverage
    artifact_id: FR-270-AC-1
    related_ids: [FR-270-AC-3]
    path: xtask/src/checked_input.rs
    lines: "1184-1192"
    excerpt: |-
      and a pub fn in qsl-replay outside spine taking a quire-contract-model package, each make the gate exit non-zero with one signature finding
    finding: "No plant mixes an exempt name with a node-bearing quire-contract-model type and expects one finding."
bindings:
  - {test_id: TC-745, ac_id: FR-270-AC-3, trace: correct}
  - {test_id: TC-161, ac_id: FR-063-AC-7, trace: correct}
```

+++
