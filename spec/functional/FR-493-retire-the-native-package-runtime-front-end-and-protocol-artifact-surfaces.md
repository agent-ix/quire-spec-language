---
id: FR-493
title: "Retire the native package, runtime-input, front-end and protocol-artifact surfaces"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-004
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: references
  - target: ix://agent-ix/quire-spec-language/ADR-012
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-027
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-100
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-490
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-049
    type: references
---
# FR-493: Retire the native package, runtime-input, front-end and protocol-artifact surfaces

## Description

QSL runs a source only through the spine stages `check`, `package` and
`execute`. Following ADR-012 §15.8 step 2 and ADR-011 §7.3 M-6c, QSL SHALL
remove the native compiled-package, runtime-input and `0-draft` front-end
paths and the compiled-protocol read side, each in the change that makes it
unreachable, so that no two paths to the same behavior exist side by side.
[FR-490](FR-490-retire-the-legacy-mapped-extraction-surface.md) retires the
mapped and extraction adapter; the state evaluator and `lower` are retired
([FR-049](FR-049-admit-composed-evaluation-inputs.md)); this requirement
retires the rest.

The behavior these paths provided has a replacement on the spine, except where
this requirement says it has none:

| Retired path | Replacement |
| --- | --- |
| A `NativePackage` and the `native-linked-package/1` document | `quire.checked-package/v2`, produced by [FR-027](FR-027-export-compiled-native-package.md) and read by QSL's I2 reader ([FR-264](FR-264-emit-and-read-v2-packages-at-schema-fixed-depth.md)) |
| Native runtime-input artifacts (`native-state-input/1`) and their construction, byte intake and validation | Snapshot and invocation admission by [FR-106](FR-106-admit-snapshots-and-invocations.md), and the clause environment of [FR-107](FR-107-evaluate-state-clauses-at-s6a.md) |
| Native selected-package execution and `runtime::execute` | The spine clause run of [FR-109](FR-109-run-a-state-clause-through-the-spine.md) and function run of [FR-100](FR-100-run-a-named-function-through-the-spine.md) |
| Native `compile` of a `0-draft` source | None. The `0-draft` edition and the native rule-model authoring profile are retired as a whole; a `0-draft` source refuses `unknown_edition` ([FR-027-AC-7](FR-027-export-compiled-native-package.md), [FR-100-AC-2](FR-100-run-a-named-function-through-the-spine.md)) |
| The `0-draft` parser, native syntax arena, native linking and checking, `formal_source`, `model_source` and `located_json` | The CST parser, S2 forms, S3 checker and S4 emission of the spine ([FR-362](FR-362-produce-the-checked-graph-and-the-type-report-from-one-source.md)) |
| The compiled-protocol v1, v2 and v3 read side (`protocol_artifact::{read, v2, v3, wire, validate}`) | Protocol nodes of `quire.checked-package/v2`, emitted by [FR-383](FR-383-emit-a-checked-protocol-as-v2-protocol-nodes.md) and read by I2 |

## Behavior

- The root QSL package SHALL have no `package` module, `NativePackage` type or
  `native-linked-package/1` wire format, and no schema, example, fixture or
  document that describes them.
- The root QSL package SHALL have no `runtime` module, no construction, byte
  intake or validation of native runtime-input artifacts, no
  `native-state-input/1` wire format, and no schema or document that describes
  them.
- When the root command receives a `compile` request whose source declares
  `edition "0-draft"`, QSL SHALL refuse it with `unknown_edition` as FR-027 and
  FR-100 specify, and SHALL NOT compile, lower or run it by any other route.
- The root QSL package SHALL have no `0-draft` parser entry point, native
  syntax arena, native linking or checking stage, `formal_source`,
  `model_source` or `located_json` module, and no build gate, example or
  Makefile target SHALL invoke one.
- The root QSL package SHALL have no module that reads or validates
  compiled-protocol v1, v2 or v3 bytes, and no documentation of those formats.
- QSL SHALL contain no compatibility alias, re-export, shim, fallback or
  feature-gated duplicate of any retired item, and SHALL delete a retired path
  in the same change that makes its last caller unreachable, never before the
  replacement above works.
- Every requirement and test case that only describes a retired path SHALL be
  removed with that path, and every acceptance criterion that a retired test
  alone backed SHALL be removed or re-owned by its replacement; the retained
  requirements keep their tests.

## Retired requirements

The change that deletes the native package and runtime-input paths removes the
requirements below, because each describes only those paths. Where a retained
requirement cites one of them, it cites the replacement named here.

| Retired | Describes | Cited elsewhere as | Replacement |
| --- | --- | --- | --- |
| FR-007 | Native runtime snapshot and invocation validation | the input-validation obligation | FR-106 (admission), FR-107 |
| FR-008 | The native reference state evaluator | the exact-budget and independent-oracle criteria (FR-008-AC-16) | FR-047 and FR-049-AC-7 (exact and one-short budgets) |
| FR-018 | Construction of native runtime artifacts | the four-label artifact identity | FR-106 |
| FR-024 | Byte intake of native runtime artifacts | the artifact reader | FR-106 |
| FR-028 | Execution of a selected native package | the selected-package run | FR-027, FR-109 |
| NFR-006 | Bounds on native runtime work | the native accounting contract | NFR-009 |
| TC-055 to TC-066, TC-099, TC-100, TC-431 | Runtime-artifact construction, intake and validation | | TC-465 and TC-136 |
| TC-067 to TC-077 | The native reference evaluator and its API workflow | | TC-136, TC-137, TC-1048 |
| TC-106 | Selected-package execution | | TC-469 |
| IT-006 | The native source-to-reference-result pipeline | | TC-469 |

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-493-AC-1 | A downstream Rust crate cannot import the root package's `package` or `runtime` modules, `NativePackage`, or `runtime::execute`; no tracked file under `src/`, `tests/`, `schemas/`, `docs/` or `examples/` holds the text `native-linked-package/1` or `native-state-input/1`, and the `WireFormat` of `qsl-foundation` has no variant for either. | Test (TC-1276) |
| FR-493-AC-2 | A `compile` request whose source declares `edition "0-draft"` refuses `unknown_edition` at stage `profile` with exit code 20 and no package bytes; the `1-draft` compile cases are unchanged. | Test (TC-1276) |
| FR-493-AC-3 | A downstream crate cannot import the root package's `0-draft` parser entry points, native syntax arena, native linking or checking modules, `formal_source`, `model_source` or `located_json`; no Makefile target, `xtask` lane or example invokes the retired parser. | Test (TC-1276) |
| FR-493-AC-4 | A downstream crate cannot import a root module that reads or validates compiled-protocol v1, v2 or v3 bytes; `docs/compiled-protocol-v1.md` and `docs/compiled-protocol-v2.md` are absent; a protocol-bearing package still reads through the I2 reader and round-trips as TC-1063 specifies. | Test (TC-1276, TC-1063) |
| FR-493-AC-5 | After the retirement, `quire matrix` over the repository lists no acceptance criterion of a retained requirement without a tagged test because its test was deleted with a retired path, and no requirement, test case or trace tag names a retired requirement of the table above. | Test (TC-1276) |

## Dependencies

- [ADR-011](../decisions/ADR-011-stage-dag-and-dependency-architecture.md) §7.3
  and [ADR-012](../decisions/ADR-012-semantic-family-extension-contracts.md)
  §15.8 authorize the deletion, one path per change.
- [FR-027](FR-027-export-compiled-native-package.md) and
  [FR-100](FR-100-run-a-named-function-through-the-spine.md) own the
  `unknown_edition` refusal; [FR-049](FR-049-admit-composed-evaluation-inputs.md)
  owns the checked-term replacement for the retired state evaluator.
