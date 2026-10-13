---
id: FR-032
title: "Run the named ConfigVersion workflow"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-002
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-025
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-031
    type: references
---
## Description

When an author generates the ConfigVersion example, the Rust generator shall emit real model, source and observation files that execute the named parent and update workflow through the clause spine (FR-312). Its results use the native-run-result/2 contract (FR-267).

## Inputs

An explicitly selected output directory and a newly authored, checked-in
AGPL-3.0-only native model source at examples/config-version/model.json, with no
copied or generated third-party model content. The model declares ConfigVersion.versionNumber as signed 0..1000,
an optional identity-bearing parent reference, explicit config_history universe,
and attemptUpdate returning Boolean with permission to change versionNumber only.
This is A's concrete native realization of the reviewed example, using the
public model frontend. It does not relabel historical Filament datatype bytes
as formal semantics or require fresh TypeSpec/Node execution. The example's
finite populations are explicit runtime inputs; no new population-bound model
feature or backend numeric/object support is claimed.

## Outputs

Per-case model/program/snapshot/invocation files, clause requests and
native-compile/1 requests with actual selected byte digests and explicit authored,
native/formal and runtime identities. Extracted Markdown units retain the
original document mapping. Existing historical standard fixture bytes remain
unchanged. The new inputs and their expected judgments are example data; only
actual clause reports constitute execution observations.

## Behavior

The generator shall compile its model with the existing model source frontend and construct runtime artifacts with the public Rust APIs.
The generator shall preserve optional absence, object identity, reference closure and pre/post observations in the emitted cases.
The generator shall use fresh case-specific snapshot and invocation identities.
If model or runtime construction or filesystem output fails, then the generator shall return an error and exit with code 2.
The generated parent-order cases shall cover healthy, violating and absent-parent outcomes.
The generated graph/identity cases shall cover a cycle, self-loop and equal-valued distinct objects.
The generated update cases shall cover unchanged version, changed version and a forbidden parent-field change.
The generated adverse cases shall cover dangling parent, incomplete population, missing model and exhausted runtime work.
The clause spine shall evaluate the emitted requests using its compiler,
admission and evaluation paths, and FR-267 shall preserve their result fields
across a native-run-result/2 write and read.

The declared case catalog supplies the complete generation list and named input
data; independent exhaustive test expectations own the expected outcomes.
Missing-model compilation refuses before evaluation. Other listed cases retain
their clause-report disposition, including admission refusals and work
exhaustion. Extracted Markdown execution retains its distinct source identity.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-032-AC-1 | The actual model frontend admits ConfigVersion with exact integer, optional-reference and operation-frame roles; the clause spine gives the independently expected parent, cycle and identity judgments across the 17-case corpus. | Test (TC-110, TC-469) |
| FR-032-AC-2 | The generated pre/post observations give `evaluate/success/true` for an unchanged version, `evaluate/violation/false` for a permitted version change and `admit/refusal/frame_violation` for a forbidden parent change, retaining the case identities. | Test (TC-469) |
| FR-032-AC-3 | Dangling/incomplete populations, missing model and exhausted work give their independently expected refusal or incomplete stage and code without Boolean truth; a fresh healthy clause request succeeds. | Test (TC-469) |
| FR-032-AC-4 | A request pinned to the emitted package id gives the same clause report; direct and Quire-extracted runs agree on stage, category, truth and exit code, and the extracted run carries the package id independently compiled from its body. The boundary refusal's object and field survive an FR-267 native-run-result/2 write and read. | Test (TC-469) |

## Dependencies

- [FR-025](FR-025-compile-rule-model-source.md): concrete native model compilation.
- [FR-267](FR-267-write-run-results-as-native-run-result-2.md): closed result writer and reader.
- [FR-312](FR-312-read-a-clause-run-request-and-run-it.md): clause runner.
