---
id: FR-018
title: "Construct bounded exact native runtime artifacts"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-003
    type: traces_to
---
# FR-018: Construct bounded exact native runtime artifacts

## Description

When a native runtime draft is supplied, the input constructor shall produce an immutable byte-bound artifact after its structural checks succeed.

## Inputs

SnapshotDraft or InvocationDraft, SourceIdentity labels and ArtifactLimits.
The labels are the four of [FR-001](FR-001-read-exact-source.md): the caller
names the artifact by authority, identity, revision namespace and revision
value, as QSpec FR-004 requires of every immutable key. A runtime artifact is
not a source read under FR-001; it only shares the label set.
The complete variant inventory, fields and native-state-input/1 encoding are
defined in [the input contract](../../docs/native-runtime-inputs.md). Flat arena
nodes use typed local ValueIds; object references use exact model/type/universe/key
identity. This requirement owns construction; the separate
[FR-024](FR-024-read-native-runtime-artifacts.md) owns external byte intake.

## Outputs

Immutable Snapshot or Invocation with exact emitted bytes, SHA-256 ByteDigest
and role-specific reference, or InputError with a structured draft location,
stable code and actual usage. No fabricated authored span or partial artifact
is returned. Construction does not establish runtime validity or completeness.

## Behavior

All child edges refer to earlier nodes in their own artifact. All nodes receive
bounded structural inspection, including unused nodes. Primitive nominal types
are determined later by their model sites; record/enum nodes name exact owners.
Field/object duplicate vectors remain available for model-aware diagnosis.
Labels and all content enter deterministic complete bytes; expected references
can select an exact digest. Limits and accounting follow
[NFR-006](../non-functional/NFR-006-bound-native-runtime.md).

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-018-AC-1 | Valid flat drafts construct immutable artifacts preserving all roots, nodes, duplicate entries and vector order. | Test |
| FR-018-AC-2 | Forward, self and out-of-range child/root indices refuse with invalid_runtime_input and no artifact; ValueIds have artifact-local meaning. | Test |
| FR-018-AC-3 | Identical labels/content reproduce exact bytes/digests; changed labels or content produce the independently expected byte artifact. | Test |
| FR-018-AC-4 | Snapshot and invocation references retain distinct roles and exact labels/digests; empty identity or revision labels refuse. | Test |
| FR-018-AC-5 | Structural checks include unused nodes and preserve invalid model-dependent shapes for validation rather than manufacturing a successful runtime judgment. | Test |
| FR-018-AC-6 | Byte, node, entry and depth limits admit exact work and refuse before the next unit, including zero/lowered ceilings and attempts to raise hard ceilings. | Test |
| FR-018-AC-7 | Construction failures identify the actual draft path/labels and leave caller-retained input unchanged without a fabricated native source span. | Test |
| FR-018-AC-8 | A snapshot constructed under (`agent-ix`, `s`, `git`, `1`) emits native-state-input/1 bytes naming all four labels, and its reference retains them; constructing it with an empty or blank authority or revision namespace refuses with `invalid_source_identity`, the code an empty identity or revision label refuses with today (FR-018-AC-4). Two snapshots that differ only in revision namespace have different bytes and digests. | Test (TC-431) |

## Dependencies

- [US-003](../usecase/US-003-evaluate-bounded-state.md) supplies the operator need.
- [FR-007](FR-007-validate-runtime-inputs.md) consumes artifacts without trusting construction as model validation.
- Existing SourceIdentity, ByteDigest, IR RequirementRef/SymbolName and serde_json/sha2 dependencies are reused.

## Status

FR-018-AC-8 is implemented under QSL-233 (ADR-013 §7 slice S-4b) and backed by TC-431.

Qualified construction API at c8fa41f, reviewed in SR-096. TC-055–057 pass with
21 public API tests and a role-separation compile-fail doctest. Model-aware
validation and reference execution are qualified by SR-097 and SR-098;
Task-015 owns the native API review/handoff. B retains portable ArtifactRef
and result envelopes; this contract creates no new portable authority, semantic
digest or input decoder.
