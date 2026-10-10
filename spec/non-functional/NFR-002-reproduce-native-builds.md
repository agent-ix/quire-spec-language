---
id: NFR-002
title: "Reproduce and validate native builds from locked inputs"
type: NFR
quality_attribute: portability
relationships:
  - target: "ix://agent-ix/quire-spec-language/FR-001"
    type: constrains
  - target: "ix://agent-ix/quire-spec-language/FR-002"
    type: constrains
  - target: "ix://agent-ix/quire-spec-language/FR-010"
    type: constrains
---
# NFR-002: Reproduce and validate native builds from locked inputs

## Statement

When the recorded native build command is run, the repository shall build without a Node or JVM runtime dependency.

When the local aggregate CI gate is invoked, the repository shall complete
native structural validation of its specification before starting any Cargo
process belonging to that aggregate, including under parallel Make execution.

## Scope

Rust crate, lockfile, toolchain and CI.

## Measurement and Evaluation

| Metric | Target | Threshold | Method |
| --- | --- | --- | --- |
| Direct Rust dependencies absent from manifest/lockfile declarations | 0 | 0 | Manifest/lock inspection |
| Required Node/JVM processes in native parse or format | 0 | 0 | CLI execution |
| Required optional Cargo features | 0 | 0 | Minimal-feature build |

## Verification

Run the recorded toolchain with the locked minimal-feature build, tests, formatter and Clippy.

Execute [TC-920](../test-cases/TC-920-native-spec-validation-precedes-ci-cargo.md)
as Rust controls using real Make and native Quire. Its Cargo process recorder
measures invocation at the aggregate boundary; it does not qualify compilation.
Run the default full scoped validation over the actual repository separately
from the deliberately invalid owned fixtures.

During stabilization, checks run locally. Every hosted CI workflow shall expose
only `workflow_dispatch`; push, pull-request, schedule and other automatic
triggers are disabled. A later return to automatic CI requires the owner's
direction and specification review. Manual availability does not authorize an
agent to dispatch a hosted run. Inspect all workflow event declarations and
record local commands, revisions and outcomes; absence of a hosted run is not a
passing check.

## Acceptance Criteria

These criteria govern the local CI validation policy; the build measurements
above remain applicable.

| ID | Criteria | Verification |
| --- | --- | --- |
| NFR-002-AC-1 | The default structural gate selects every Markdown document under spec/ recursively from the caller's repository root and validates it through installed scoped module discovery, without narrowing to one module or suppressing errors. A valid selected corpus succeeds. | Test |
| NFR-002-AC-2 | With a structurally invalid selected document, make ci returns nonzero and starts zero Cargo processes, for both serial and parallel execution, including parallel keep-going execution. With the document restored, the same executing process-boundary control observes Cargo calls. | Test |
| NFR-002-AC-3 | Missing native Quire, missing discovered modules, an unmatched document selection, or malformed required frontmatter, body sections or table headers makes the structural gate fail. The gate retains native diagnostics and actual native exit status in Make's error report and does not install modules to turn configuration failure into success. | Test |
| NFR-002-AC-4 | Owned Rust gate fixtures resolve an unset Cargo target to the workspace target directory, a relative target from the workspace root, and an absolute target unchanged; each configuration creates and cleans up its fixture there and executes the native validator, including paths containing spaces. Actual assertions use imported canonical TC and AC trace attributes. | Test |

## Dependencies

- [FR-001](../functional/FR-001-read-exact-source.md)
- [FR-002](../functional/FR-002-parse-native-units.md)
- [FR-010](../functional/FR-010-report-native-outcomes.md)

## Verification language policy

[NFR-005](NFR-005-rust-verification-paths.md) additionally requires Rust for owned
verification logic.
