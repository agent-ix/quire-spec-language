---
id: SR-1933005
title: "QSL-1359 code-review of conformance selector repair"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; QSL-1359; xtask/src/conformance.rs, xtask/src/conformance/tests.rs, xtask/tests/fixtures/checks.rs, xtask/tests/fixtures/ci_conformance_cargo_double.rs, qsl-package/src/checked_v2/tests.rs"
review_set: subset
relationships:
  - {target: "ix://agent-ix/quire-spec-language/FR-088", type: references}
---

## Summary

Reviewed frozen acf5c3dd976f4fb276172d74afde98d5292ad03a, origin/main...HEAD (five Rust files). Ticket: QSL-1359. The assigned scope is repair of the existing conformance selector gate, not a repository-wide assurance campaign. No applicable AssuranceProfile was found. No production semantic implementation or workflow changes are introduced.

The four deleted selectors have no direct replacements: application preimage hashes, nominal-enum digest keys, compound-unit digest vectors, and identity_projection envelope digest admission were deleted by structural migration 29f2a877a (#1885). The removed compound test asserted SHA-256 bytes; current FR-088-AC-12 instead requires factor-record structural equality and unit separation, exercised by the two surviving quantities tests. FR-092 explicitly removes the preimage/digest contract. Removing these selectors therefore removes references to retired assertions rather than relabeling an unrelated test as equivalent.

All six remaining CHECKS entries have real functions. check_output still requires exactly one passed, zero failed, zero ignored; checks child status, explicit skip markers, and positive vector counts. Every remaining selection is exercised by the failure-mode loop. The renamed zero-vector test feeds C14 zero source-map entries; the replacement fixture cannot satisfy its positive-count parser. The frame test remains ignored, its required gate entry remains present, and the assertion change only adds the structured refusal to failure output.

Directly read designated QSpec FR-322 Operand index segments and FR-340-AC-3. The former requires closed {node:i} path segments; locked IR 8320ebe7 predates that representation. The latter requires invalid_model_binding/malformed-declaration for a literal relationship target once wire is valid. Existing actual MalformedWire at /semantic_graph/nodes/2/node_id occurs before frame semantics, so changing the oracle or unignoring the test would be unjustified. The dirty designated QSpec checkout was read only; no merged fixture release is claimed.

## Verdict

PASS within the assigned diff and existing-gate repair scope. No introduced defect or unsupported selector replacement found. The repository aggregate gate remains failing on the separately documented reader incompatibility.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Coverage

Independent cargo fmt --all -- --check passed. Independent cargo test --locked -p xtask conformance::tests passed all seven process tests (process-review.log); process doubles are gate evidence, not QSpec semantic evidence. Read supplied clippy-final.log (focused xtask/qsl-package Clippy passed), semantics.log (three real semantics tests passed), retained.log, and frame-final.log (real ignored frame test fails the documented early wire stage). Read lead-owned /tmp/codex1-qsl-1359-gate.log: make ci exits 2 at retained C14, MalformedWire /semantic_graph/nodes/2/node_id. No aggregate rerun. Full CI is still failing; this review does not authorize describing qualification as passing.

## Examined units

```yaml
{
  "scope": [
    {
      "id": "FR-088-AC-12",
      "path": "spec/functional/FR-088-clause-name-and-type-identity.md",
      "role": "examined",
      "excerpt": "For declared units `m` and `km` (a scaled unit of `m`) and the compound unit `m^1`, the three `UnitId`s are pairwise unequal. `m`'s `UnitId` carries `m`'s node key under the `quire.checked-semantic-node/v1` label. For every compound unit, the compound `UnitId` equals its ordered (unit node key, exponent) factor record by structural equality. C-30 refuses a dimension node key as a declared-arm `UnitId`."
    },
    {
      "id": "FR-092-AC-4",
      "path": "spec/functional/FR-092-key-type-parameter-and-declared-nodes.md",
      "role": "examined",
      "excerpt": "Functions with two parameters, no parameters, a `let` binder and an integer literal construct the parameter, function and literal nodes and references specified above. Their ids have FR-322 structural components and no preimage bytes or digest."
    },
    {
      "id": "xtask/src/conformance.rs",
      "path": "xtask/src/conformance.rs",
      "role": "examined",
      "excerpt": "    Check { package: \"qsl-package\", target: Target::Lib, selection: \"checked_v2::tests::conformance_c14_source_map_lookup_over_qspec_positive_fixtures\", summaries: &[&[\"conformance: \", \" source-map entries over \", \" positive fixtures\"]], summary_mode: SummaryMode::Exact },\n    Check { package: \"qsl-package\", target: Target::Lib, selection: \"checked_v2::tests::conformance_i2_read_over_qspec_checked_package_v2_fixtures\", summaries: &[&[\"conformance: \", \" positive fixtures through QSL's full I2 read\"], &[\"conformance: \", \" adverse mutations refused\"]], summary_mode: SummaryMode::Prefix },\n    Che"
    },
    {
      "id": "xtask/src/conformance/tests.rs",
      "path": "xtask/src/conformance/tests.rs",
      "role": "examined",
      "excerpt": "    assert!(!workspace.join(\"calls\").exists());\n}\n\n#[test]\nfn each_selection_refuses_missing_evidence_or_failure() {\n    for (index, _) in CHECKS.iter().enumerate() {\n        for mode in [\"zero-tests\", \"ignored\", \"skip\", \"no-summary\", \"nonzero\"] {\n            let root = crate::test_support::tempdir();\n            let checkout_path = root.path().join(\"quire-specification\");\n            checkout(&checkout_path);\n            assert!(matches!(\n                run_case(root.path(), checkout_path.as_os_str(), mode, index),\n                Err(Error::ConformanceCheck { .. })\n            ));\n         "
    },
    {
      "id": "xtask/tests/fixtures/checks.rs",
      "path": "xtask/tests/fixtures/checks.rs",
      "role": "examined",
      "excerpt": "pub(super) const CHECKS: [(&str, &str); 6] = [\n    (\"checked_v2::tests::conformance_c14_source_map_lookup_over_qspec_positive_fixtures\", \"conformance: 1 source-map entries over 1 positive fixtures\"),\n    (\"checked_v2::tests::conformance_i2_read_over_qspec_checked_package_v2_fixtures\", \"conformance: 1 positive fixtures through QSL's full I2 read\\nconformance: 1 adverse mutations refused\"),\n    (\"checked_v2::tests::conformance_fr340_frame_mutations_match_qspec_vectors\", \"conformance: 1 frame-body mutation vectors matched\"),\n    (\"checked_v2::tests::conformance_dependency_selection_vectors\", \"con"
    },
    {
      "id": "xtask/tests/fixtures/ci_conformance_cargo_double.rs",
      "path": "xtask/tests/fixtures/ci_conformance_cargo_double.rs",
      "role": "examined",
      "excerpt": "    match mode.as_str() {\n        \"no-summary\" => {}\n        \"zero-vectors\" => {\n            println!(\"conformance: 0 source-map entries over 1 positive fixtures\")\n        }\n        _ => println!(\"{}\", CHECKS[index].1),\n    }\n    match mode.as_str() {"
    },
    {
      "id": "qsl-package/src/checked_v2/tests.rs",
      "path": "qsl-package/src/checked_v2/tests.rs",
      "role": "examined",
      "excerpt": "        match outcome {\n            Read::Refused(V2ReadRefusal::Envelope { refusal, .. }) => {\n                assert_eq!(refusal.code, expected_code, \"{name}: code ({refusal:?})\");\n                assert_eq!(refusal.cause, expected_cause, \"{name}: cause\");\n                assert_eq!(\n                    refusal\n                        .locus\n                        .as_ref()"
    },
    {
      "id": "QSpec-FR-340-AC-3",
      "path": "/Users/peter/dev/quire-specification-qsl1314/spec/objects/interfaces/FR-340-checked-package-frame-location.md",
      "role": "examined",
      "excerpt": "An entry naming no node of the package, or no declared dependency of the frame node, refuses `missing_declaration`/`missing-name`; an entry naming a declared dependency of any other meaning -- a relationship entry naming an object type or a literal, a field entry on a relationship, a persistence interface or a systems node, a `creates` entry naming a relationship, and every `model`/`*_type`, `model`/`namespace` or `model`/`persistence_interface` form an entry cannot carry, and a `creates` or `deletes` entry naming a `model`/`state_machine` -- refuses `invalid_model_binding`/`malformed-declarat"
    },
    {
      "id": "QSpec-FR-322-operand-index-segments",
      "path": "/Users/peter/dev/quire-specification-qsl1314/spec/objects/interfaces/FR-322-checked-package-artifact.md",
      "role": "examined",
      "excerpt": "An operand node is never written inside another node's id. The wire holds it\nas an *index segment*, the closed object `{\"node\": i}` whose one member `node`\nis a JSON integer from `0` to 9007199254740991 naming `semantic_graph.nodes[i]`\nby its graph index. Any other segment object (a `node_id` object, an extra\nmember, a missing `node`, a string, negative, fractional or exponent-form\n`node`) fails the V2 schema and refuses `malformed_wire`. So a `node_id` has a\nfixed JSON depth (the object, its `path` array and an index segment), whatever\nthe depth of the expression it names, and its size is lin"
    }
  ]
}
```
