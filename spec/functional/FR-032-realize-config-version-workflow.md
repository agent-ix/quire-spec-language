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

When an author generates the ConfigVersion example, the Rust generator shall emit real model, native/Markdown source and runtime files that execute the named parent and update workflow through the standalone command.

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

Per-case model/program/snapshot/invocation files, native-run/1 requests and
native-compile/1 requests with actual selected byte digests and explicit authored,
native/formal and runtime identities. Markdown run requests use FR-031 and retain
the original document mapping. Existing historical standard fixture bytes remain
unchanged. The new inputs and their expected judgments are example data; only
actual command results constitute execution observations.

## Behavior

The generator shall compile its model with the existing model source frontend and construct runtime artifacts with the public Rust APIs.
The generator shall preserve optional absence, object identity, reference closure and pre/post observations in the emitted cases.
The generator shall use fresh case-specific snapshot and invocation identities.
If model or runtime construction or filesystem output fails, then the generator shall return an error and exit with code 2.
The generated parent-order cases shall cover healthy, violating and absent-parent outcomes.
The generated graph/identity cases shall cover a cycle, self-loop and equal-valued distinct objects.
The generated update cases shall cover unchanged version, changed version and a forbidden parent-field change.
The generated adverse cases shall cover dangling parent, incomplete population, missing model and exhausted runtime work.
The standalone command shall evaluate the emitted requests using its existing compiler, validation and runtime paths.

The declared case catalog supplies the complete generation list and named input
data; independent exhaustive test expectations own the expected outcomes.
Missing-model compilation fails before a runtime report exists. Other listed
cases retain runtime report provenance, including validation refusals and work
exhaustion. Markdown execution of those cases retains extraction provenance;
its missing-model failure instead retains the original selection in error details.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-032-AC-1 | The actual model frontend admits ConfigVersion with exact integer, optional-reference and operation-frame roles, and the real binary returns the independently expected parent, cycle and identity judgments. | Test |
| FR-032-AC-2 | Real pre/post invocation files produce true for an unchanged version, false for a permitted version change and frame_violation for a forbidden parent change, retaining exact case identities. | Test |
| FR-032-AC-3 | Dangling/incomplete populations, missing model and exhausted work produce their actual refused/incomplete stage and code without Boolean truth; a fresh healthy request succeeds. | Test |
| FR-032-AC-4 | Exported native packages reconstruct the same results; native and actual Quire-extracted executions agree on logical outcomes while retaining their distinct source/package identities and exact original/body mapping. | Test |

## Dependencies

- [FR-025](FR-025-compile-rule-model-source.md): concrete native model compilation.
- [FR-026](FR-026-run-standalone-native-workflow.md): real local file execution.
- [FR-031](FR-031-run-extracted-native-source.md): actual Markdown extraction mode.
