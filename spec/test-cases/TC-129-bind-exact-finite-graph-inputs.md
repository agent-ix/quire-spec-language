---
id: TC-129
title: "Bind exact finite graph identities and inputs"
type: TC
relationships:
  - { target: ix://agent-ix/quire-spec-language/FR-047, type: verifies }
---
# TC-129: Bind exact finite graph identities and inputs

## Description

Verify that graph compilation and evaluation bind exact typed object/reference
identities, observations and independently supplied closure authority.

## Test Procedure

Compile and read a graph predicate over two same-shaped models and universes.
Supply complete current and pre-state environments through the public immutable
input API and dereference the selected typed reference. Include the same logical
model/universe/type/object identity once under each of the distinct pre and post
observation occurrences, and twice at one semantic anchor under distinct record
identities. Mutate one axis at a time: anchor, snapshot, optional window, record,
model, universe, object type, object identifier and edge owner. Substitute a
compiled-artifact reference or raw-byte digest for one producer/observation
identity or canonical digest. Then duplicate one complete `(observation
occurrence, model, universe, object type, object identifier)` storage key.

Under declared-complete membership and closure, remove an exact reference target.
Separately mark membership or closure unavailable while offering the same absent
target. Supply a known foreign/wrong-type reference both with and without
completeness so structural invalidity is not hidden by missing authority.

## Expected Results

The positive case returns the exact selected object and observation occurrence.
Distinct anchored or recorded copies do not collide, and dereference never
retags one as another. A duplicate full key, a target missing from a
declared-complete domain, a digest-domain substitution, or a known identity/
type/authority mutation refuses with a distinct typed cause. Missing membership
or closure is incomplete and does not classify the offered subset as empty,
closed or dangling. No string identifier or ambient lookup repairs a mutation.
