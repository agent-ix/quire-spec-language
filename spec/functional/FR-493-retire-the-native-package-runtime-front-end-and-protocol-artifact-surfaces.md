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
  - target: ix://agent-ix/quire-spec-language/FR-362
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-042
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-050
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
retires the rest of the SEAM-1 native-v1 lane (ADR-011 §7.3) and the compiled-protocol
read side of SEAM-3. It does not retire SEAM-2, the composed lane
(`syntax::composed`, `parser::composed`, `linking::composed`,
`checking::composed` and `formal_source`, which `checking::composed` imports),
which stays normative and is deleted family by family under
[FR-362](FR-362-produce-the-checked-graph-and-the-type-report-from-one-source.md)
item 9 (ADR-012 §15.8 step 2: `formal_source` is not deleted in step 2); and it
does not delete `native_model`, which SEAM-2 and SEAM-3 still import
(ADR-012 §15.8 step 3).

The behavior these paths provided has a replacement on the spine, except where
this requirement says it has none:

| Retired path | Replacement |
| --- | --- |
| A `NativePackage` and the `native-linked-package/1` document | `quire.checked-package/v2`, produced by [FR-027](FR-027-export-compiled-native-package.md) and read by QSL's I2 reader ([FR-264](FR-264-emit-and-read-v2-packages-at-schema-fixed-depth.md)) |
| Native runtime-input artifacts (`native-state-input/1`) and their construction, byte intake and validation | Snapshot and invocation admission by [FR-106](FR-106-admit-snapshots-and-invocations.md), and the clause environment of [FR-107](FR-107-evaluate-state-clauses-at-s6a.md) |
| Native selected-package execution and `runtime::execute` | The spine clause run of [FR-109](FR-109-run-a-state-clause-through-the-spine.md) and function run of [FR-100](FR-100-run-a-named-function-through-the-spine.md) |
| Native `compile` of a `0-draft` source | None. The `0-draft` edition and the native rule-model authoring profile are retired as a whole; a `0-draft` source refuses `unknown_edition` ([FR-027-AC-7](FR-027-export-compiled-native-package.md), [FR-100-AC-2](FR-100-run-a-named-function-through-the-spine.md)) |
| The `0-draft` parser, native syntax arena, `linking::native`, native `checking`, `model_source` and `located_json` | The CST parser, S2 forms, S3 checker and S4 emission of the spine ([FR-362](FR-362-produce-the-checked-graph-and-the-type-report-from-one-source.md)) |
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
- The root QSL package SHALL have no `0-draft` parser entry point, arena
  `syntax`, `linking::native`, native `checking`, `model_source` or
  `located_json` module, and no build gate, example or Makefile target SHALL
  invoke one.
- QSL SHALL keep `formal_source`, `native_model` and every `composed` module
  until the slice that removes their last importer, as ADR-012 §15.8 steps 2
  and 3 and FR-362 item 9 order; this requirement neither deletes nor
  forbids them.
- The root QSL package SHALL have no module that reads or validates
  compiled-protocol v1, v2 or v3 bytes, and no documentation of those formats.
  The strict reader is called by the handoff writers
  (`protocol_artifact::handoff::write_v1` and `write_v2`,
  [FR-042](FR-042-publish-compiled-protocol-artifacts.md)-AC-15 and
  [FR-050](FR-050-publish-authenticated-temporal-artifacts.md)-AC-8) and by the
  composed emission, so QSL SHALL delete it in the slice that deletes the last
  of them (M-6d, ADR-011 SEAM-3), not before.
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

## Front-end and protocol-artifact requirement dispositions

Each requirement below describes a path this requirement retires in whole,
in part, or not at all. A criterion marked **Retire** is removed, with its test
cases, in the slice that deletes the path; a criterion marked **Keep** stays
normative. No requirement is deleted before its path is unreachable.

| Requirement | Disposition |
| --- | --- |
| FR-001 (TC-011, TC-424) | **Keep** all live criteria (AC-3 to AC-12): the spine admits every source through the one `SourceIdentity`. AC-1 and AC-2 stay retired. The description's native-v1 sentences and the "Runtime artifacts are not sources" paragraph name FR-018 and FR-024; the paragraph cites FR-106 instead. |
| FR-016 (TC-025 to TC-029, TC-046 to TC-053, TC-895) | **Retire** AC-1 to AC-10 with native `checking` and `linking::native`; FR-104 and FR-107 own the spine checker and evaluator. Two retained requirements cite it and are re-pointed in the same slice: FR-104's `CheckBindings`/`ClauseBinding` adapter paragraph (the native route that validates a `Vec<ClauseBinding>` is deleted, so the execution-point map has no native producer) and FR-098's two citations of the short-circuit and conditional order (to FR-107). |
| FR-017 (TC-054, TC-030 to TC-034) | **Retire** AC-1 and AC-2 (the rule-model JSON decoder) with `model_source` and `located_json`, and AC-3 (native linker stage separation) with `linking::native`. |
| FR-025 (TC-101, TC-102) | **Retire** AC-1 to AC-5 with `model_source` and `located_json`. The `native-rule-model/1` source profile has no spine successor; domain-package models enter through FR-056. |
| FR-035 (TC-113), FR-036 (TC-114, TC-115, TC-148), FR-040 (TC-119) | **Keep**. These describe the composed lane (SEAM-2), which FR-362 item 9 retires family by family under QSL-868. FR-036-AC-5, which FR-049-AC-10 and TC-136 cite, stays normative. |
| FR-041 (TC-120) | **Split**. **Retire** AC-1, AC-3 and AC-4 with `model_source`, and the `link_native` refusal in AC-5 with `linking::native`. **Keep** AC-2, AC-6 and AC-7 and AC-5's composed `bind_models` clause until `native_model` is deleted last (ADR-012 §15.8 step 3), then retire them. |
| FR-042 (TC-121) | **Retire** AC-1 to AC-15 with the compiled-protocol emitters, readers and handoff writers (M-6d); AC-16 and AC-17 are already withdrawn. Its dependents FR-043 to FR-050, FR-052 and FR-053 build on this contract and need their own dispositions in that slice. FR-054 and TC-143 are already scheduled with `/3` (QSL-838, QSL-840). |

## Retained requirements the retirement changes

A requirement that stays normative cites a retired path in the places below.
The slice that deletes the path changes the citation in the same change.

| Retained | Cites | Change |
| --- | --- | --- |
| FR-100, FR-031, FR-026 | The run request `package` member and FR-028 (`extraction-package-conflict`, FR-100-AC-3's package-bearing refusal) | The member exists only to select a `native-linked-package/1`; it is removed with `NativePackage`, and a request that carries it refuses as any unknown member does. |
| FR-277, NFR-008, NFR-009 | NFR-006 as the historical native accounting contract | Delete the clause; NFR-009 and FR-255 are the accounting contract. |
| NFR-003, IT-002, US-003, StR-001 | FR-007, FR-008 and FR-018 as obligations | Cite FR-106, FR-107 and NFR-009. |
| FR-365 | FR-008-AC-16 | Cites FR-049-AC-7 (done in this change). |

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-493-AC-1 | A downstream Rust crate cannot import the root package's `package` or `runtime` modules, `NativePackage`, or `runtime::execute`; and the `WireFormat` of `qsl-foundation` has no variant for either wire format. | Test (TC-1276) |
| FR-493-AC-2 | A `compile` request whose source declares `edition "0-draft"`, or any other edition the spine does not admit, refuses `unknown_edition` at stage `profile` with exit code 20 and no package bytes; the `1-draft` compile cases are unchanged. | Test (TC-1276) |
| FR-493-AC-3 | A downstream crate cannot import the root package's `0-draft` parser entry points, arena `syntax`, `linking::native`, native `checking`, `model_source` or `located_json`; every Makefile target, `xtask` lane and example still builds and runs without it. The `composed` modules, `formal_source` and `native_model` remain importable until their own slices. | Test (TC-1276) |
| FR-493-AC-4 | In the slice that deletes the handoff writers and composed emission, a downstream crate cannot import a root module that reads or validates compiled-protocol v1, v2 or v3 bytes; a protocol-bearing package still reads through the I2 reader and round-trips as TC-1063 specifies. | Test (TC-1276, TC-1063) |
| FR-493-AC-5 | After the retirement, `quire matrix` over the repository lists no acceptance criterion of a retained requirement without a tagged test because its test was deleted with a retired path. | Test (TC-1276) |

## Dependencies

- [ADR-011](../decisions/ADR-011-stage-dag-and-dependency-architecture.md) §7.3
  and [ADR-012](../decisions/ADR-012-semantic-family-extension-contracts.md)
  §15.8 authorize the deletion, one path per change.
- [FR-027](FR-027-export-compiled-native-package.md) and
  [FR-100](FR-100-run-a-named-function-through-the-spine.md) own the
  `unknown_edition` refusal; [FR-049](FR-049-admit-composed-evaluation-inputs.md)
  owns the checked-term replacement for the retired state evaluator.
