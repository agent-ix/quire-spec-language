---
id: NFR-006
title: "Bound native runtime work and retained content"
type: NFR
quality_attribute: performance_efficiency
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-018
    type: constrains
  - target: ix://agent-ix/quire-spec-language/FR-007
    type: constrains
  - target: ix://agent-ix/quire-spec-language/FR-008
    type: constrains
---
# NFR-006: Bound native runtime work and retained content

## Statement

If the next charged runtime operation exceeds its selected ceiling, then the native runtime shall stop with resource_exhausted before performing that operation.

## Scope

Artifact construction, one validation request and one reference evaluation.
Limits are inclusive unsigned counters. Each has the published default
below; a caller sets each independently above or below its default, and the
value is used as given, with no ceiling (ADR-030 D-1).
Actual usage is returned on success and on a returned error. Exhaustion returns
no artifact, validated context or predicate Boolean for the affected stage.
The validated context remains usable for another evaluation with fresh limits.

## Measurement and Evaluation

| Metric | Target | Threshold | Method |
| --- | --- | --- | --- |
| Artifact emitted content | The selected limit; by default at most 1048576 bytes per artifact | 1048576 bytes by default | negative-abuse-testing |
| Artifact inspected text content | The selected limit; by default at most 1048576 UTF-8 bytes per artifact | 1048576 bytes by default | negative-abuse-testing |
| Artifact arena nodes | The selected limit; by default at most 100000 nodes per artifact | 100000 nodes by default | negative-abuse-testing |
| Artifact aggregate entries | The selected limit; by default at most 100000 entries per artifact | 100000 entries by default | negative-abuse-testing |
| Validation inventory | The selected limit; by default at most 64 artifacts per request | 64 artifacts by default | negative-abuse-testing |
| Validation inventory content | The selected limit; by default at most 8388608 encoded bytes per request | 8388608 bytes by default | negative-abuse-testing |
| Validation objects | The selected limit; by default at most 10000 selected object entries per request | 10000 objects by default | negative-abuse-testing |
| Validation work | The selected limit; by default at most 1000000 visits per request | 1000000 visits by default | negative-abuse-testing |
| Validation text inspection | The selected limit; by default at most 8388608 scalar advances per request | 8388608 advances by default | negative-abuse-testing |
| Validation detail diagnostics | The selected limit; by default at most 256 entries plus a separate optional terminal reason | 256 detail entries by default | negative-abuse-testing |
| Evaluation expression steps | The selected limit; by default at most 1000000 steps per run | 1000000 steps by default | negative-abuse-testing |
| Evaluation graph expansions | The selected limit; by default at most 10000 steps per run across all reaches calls | 10000 steps by default | negative-abuse-testing |
| Evaluation deep comparison | The selected limit; by default at most 100000 value pairs per run | 100000 pairs by default | negative-abuse-testing |
| Evaluation text inspection | The selected limit; by default at most 1048576 scalar advances per run | 1048576 advances by default | negative-abuse-testing |
| Evaluation event storage | The selected limit; by default at most 10000 events per run | 10000 events by default | negative-abuse-testing |

## Counter definitions

Construction checks vector lengths before allocating derived indexes. A node
costs one arena node. An entry is one model binding, population, object, object
field binding, State binding, invocation parameter, created/deleted identity,
record field or container child edge; each occurrence counts, including duplicates.
Present contributes one child edge. Root ValueIds in field/State/parameter/result
positions are checked even when no value node is reachable. Result contributes
one entry when present. Nesting depth is not a limit (ADR-030 D-1): node and
entry limits bound an artifact at any depth. Every arena node is checked,
without recursively expanding shared nodes. Object/reference identity is a leaf.

Before scanning a string, construction charges its UTF-8 byte length against an
additional aggregate 1048576-byte text-content ceiling (same artifact_bytes
option); emitted escaping still charges the independent output counter. This
prevents long rejected strings from bypassing the content work bound. All labels,
names, identity keys and payload strings participate. The exact content byte
ceilings do not describe allocator capacity or preexisting caller allocations.

Validation charges all inventory entries and encoded lengths before indexing.
Its selected-object ceiling counts every supplied object in the selected
snapshots, including duplicates and both observations of a surviving object.
Each work visit precedes one inventory entry, model binding, population, object,
field/root/parameter/delta binding, container edge, typed value use or storage
comparison pair. Separate passes and shared values used under multiple native
types/observations count separately. Population-index lookup is bounded by the
already limited inventory; it does not recursively expand referenced objects.

Text work in validation counts each Unicode-scalar iterator advance, including
the final end check; comparing strings counts each side separately. This applies
to model-bound text/key limits and frame storage equality. Index ordering of
already byte-bounded identity labels is covered by the finite entry/content
ceilings, not claimed as native expression or scalar steps. A report records
the separate validation counters, without charging input construction again.

Evaluation counters follow the exact vectors and traversal rules in
[the execution contract](../../docs/native-runtime-evaluation.md). Expression
and structural-comparison traversals run over explicit stacks, and Reaches
expands iteratively, so no traversal recurses natively in proportion to its
input. Values are borrowed; reading a large local/field does
not expand it before a comparison's counters are checked.

Poll cancellation before each charged validation/evaluation unit and loop
continuation. A true poll returns cancelled with actual previous work. A
panicking poll unwinds by ordinary Rust behavior, producing no report or partial
successful context. Construction has no poll. No claim bounds caller callbacks,
allocator failure or OS scheduling. No wall-clock SLA or thread pool is added.

## Verification

Rust generated boundary tests measure each dimension independently on a valid
fixture, admit the exact required work and stop one below or at zero when work
is required. Test each default with one-over input, and a raised limit above
the default with input that fits it and completes; where another limit
necessarily stops first, retain that coupled outcome and use a lowered
isolated limit. Empty unused dimensions can complete at zero. Invalid input with zero
diagnostic capacity retains classification and the separate terminal reason;
valid input with zero detail capacity can complete.

Use independently authored expression/graph counts, small functional-graph
closure oracles, deep shared arenas, long Unicode prefixes and deterministic
cancellation/panic controls. A repeat after successful execution must still
honor a smaller selected limit. Tests run serially with the existing Rust cache;
no timing sleep, concurrent benchmark or hosted CI run is required.

## Qualification status

TC-057 tests the construction metrics, TC-065 and TC-066 the validation
metrics, and TC-074 and TC-075 the evaluation metrics. These counters do not
establish backend fuel parity.

## Dependencies

- [FR-018](../functional/FR-018-construct-native-runtime-inputs.md), especially AC-6.
- [FR-007](../functional/FR-007-validate-runtime-inputs.md), especially AC-12/15.
- [FR-008](../functional/FR-008-evaluate-state-reference.md), especially AC-4/5/16–19.
- [NFR-005](NFR-005-rust-verification-paths.md) keeps first-party qualification in Rust.
