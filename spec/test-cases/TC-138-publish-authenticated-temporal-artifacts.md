---
id: TC-138
title: "Publish and read authenticated compiled temporal selections"
type: TC
relationships:
  - { target: ix://agent-ix/quire-spec-language/FR-050, type: verifies }
  - { target: ix://agent-ix/quire-spec-language/FR-042, type: references }
---
# TC-138: Publish and read authenticated compiled temporal selections

## Description

Verify the strict `/2` producer, wire reader and L5 selection handoff without
changing or silently upgrading the historical `/1` contract.

## Test Procedure

Compile three native temporal declarations selecting event-position,
fixed-sample and timestamped-window definitions from their exact original
dependency bytes. Supply a complete native temporal-selection table keyed by
each exact source artifact/declaration span and respectively a nonempty sequence authority, exact
epoch/positive rational period/unit, and timestamp unit. Admit emitted bytes
through `protocol_artifact::v2::read` using an independently authored complete
`v2::ExpectedTemporal` table, then evaluate each admitted declaration through
`temporal::evaluate_v2` with matching and one-field-substituted clock inputs.
Use compact canonical FR-038 Number-object JSON for the trace epoch and period;
add, omit and rename a parameter-map key independently.

Remove, duplicate, reorder and append one binding; cross its declaration and
definition indices; substitute each definition identity, revision, artifact
reference, raw byte digest and raw bytes; cross every pair of clock alternatives;
and use empty and over-byte-limit names. Exercise zero, negative, unreduced and
component-overflow periods plus a changed epoch or unit. Re-seal each mutated
candidate so the test reaches the owning validation rather than passing on an
earlier digest mismatch.

Independently remove, duplicate and append a producer temporal selection and a
reader `ExpectedTemporal`; substitute its source, declaration span, definition
or one clock field without changing the compiled source.

Run the `/1` producer and strict reader over the pre-extension fixture and
compare its bytes to the frozen expected artifact. Offer `/1` to the `/2` reader
and `/2` to the `/1` reader. For every added work dimension, independently count
the successful `/2` run and retry with exact, one-short and above-hard limits.

## Expected Results

The positive `/2` artifact has one binding per temporal declaration in canonical
declaration order; every index selects that declaration's exact definition and
original bytes, and every clock alternative matches its profile. Matching L5
inputs reach evaluation, while each clock-field substitution refuses before any
position is visited.

Every one-axis structural, identity, numeric, profile and byte substitution
returns its typed cause without an admitted partial package. Exact work succeeds,
one-short work exhausts before the next operation and a sufficient retry
reproduces the original bytes with fresh usage. The `/1` bytes remain frozen;
cross-version reads refuse, and neither path inserts, removes or infers a clock
selection.

The existing `/1` L5 entry points retain their public signatures and prior
unauthenticated-parameter results. The v2-specific entry points accept only the
v2 admitted type and retain the artifact-authenticated parameter map.
