---
id: FR-030
title: "Bind every clause to its exact semantic definitions"
type: FR
relationships:
  - target: ix://agent-ix/quire-specification/US-005
    type: implements
---
## Description

When admitting a composed native package, the frontend SHALL bind every clause to its explicitly selected language edition, profile definitions and exact model dependency closure before permitting that clause to execute.

## Inputs

Native source with exact byte/revision/span identity, clause declarations, selected definition name/revision/byte-digest tuples and imported model identities. Selection is authored or explicitly supplied by the source profile; installed backends never choose it.

## Outputs

A linked clause with resolved definition and model dependencies, or a located typed admission refusal.

## Behavior

A reused profile name with a different revision or digest is a different selection, not an alias for an installed definition. Unknown or conflicting definitions refuse all clauses depending on them. A source- or model-identity conflict prevents dependent admission; unrelated valid clauses may remain inspectable. Repeated declarations cannot introduce a second editable authority. Runtime observations and changing monitor state do not participate in static package identity.

The [package contract](../../proposals/quire-v1/package-contract.md) defines the
static components, exact import digest and typed binding requirements. The
compiler input inventory closes the native declaration namespace. Checked output
retains per-declaration dependency dispositions; partial admission is not a
complete package. Static subject identity differs from serialized artifact bytes
and build provenance; an unqualified canonical digest is not claimed.

The linker's prerequisites follow the package contract's processing-stage table.
Concrete populations, windows and observations are later assessment inputs,
not prerequisites for type-checking a valid future obligation. When a producer
supplies canonical model/profile objects, the linker retains the declared
producer/native correspondence instead of substituting that canonical digest
for a compiled-model or definition byte digest.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-030-AC-1 | A package with state, temporal and protocol clauses retains each exact selected definition and a shared exact model closure. | Test (TC-030) |
| FR-030-AC-2 | An unknown revision, digest mismatch or conflicting definition refuses dependent clauses before execution and identifies the failed selection. | Test (TC-030) |
| FR-030-AC-3 | Changing a runtime trace while keeping source, model and selected definitions fixed does not change static package identity; changing a semantic dependency does. | Test (TC-030) |
| FR-030-AC-4 | Duplicating a clause's editable authority refuses admission rather than choosing the first or last declaration. | Test (TC-030) |
| FR-030-AC-5 | An omitted required source unit prevents namespace closure; an extra file outside the explicit inventory cannot add or shadow a declaration. | Test (TC-030) |
| FR-030-AC-6 | Substituting a manifest/lock digest or package fingerprint for the compiled model import's byte digest refuses the import. | Test (TC-030) |
| FR-030-AC-7 | Linked binding roles retain owning declaration, source region, kind, exact type/contract, anchor and scope prerequisites; equal local names in different clauses do not merge their roles. | Test (TC-030) |
| FR-030-AC-8 | A compile configuration change limited to resource controls remains visible in provenance while preserving the static subject; a changed semantic selection changes the static subject. | Test (TC-030) |
| FR-030-AC-9 | A valid three-family template links with exact static model/definition/binding contracts and no runtime population, window or observations; removing a required static definition still refuses its dependent declaration. | Test (TC-030) |
| FR-030-AC-10 | A declared producer/native correspondence retains and validates each selection in its own digest domain; missing correspondence, foreign exports and cross-domain digest substitutions refuse the dependent static binding. | Test (TC-030) |

## Dependencies

- [Shared drafting foundation](../../proposals/quire-v1/shared-foundation.md).
- [Composed architecture](../assurance/AD-001-composed-native-language.md).
- [Planned acceptance matrix](../composed-foundation/tests.md).

Proposed requirement for the composed profile; it does not amend historical
accepted definitions or establish an implemented capability.
