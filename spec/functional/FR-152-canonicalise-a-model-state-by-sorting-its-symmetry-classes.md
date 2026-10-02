---
id: FR-152
title: "Canonicalise a model state by sorting its symmetry classes"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-019
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-021
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-101
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-120
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-151
    type: depends_on
---
# FR-152: Canonicalise a model state by sorting its symmetry classes

## Description

Under an admitted symmetry group, `ModelSystem` SHALL map each model state
to a canonical representative of its orbit by sorting each class's objects
by their key-free encoding and reassigning the class's keys in sorted order
(ADR-021 SYM-7). It returns the representative and the permutation that
produced it. The engine stores the representative in place of the state, so
two states coalesce exactly when their representatives have equal state-key
bytes.

## Use case

A verification operator's three interchangeable configs reach 27 states
unreduced; the canonicaliser keeps one state per multiset of versions, 10,
at a cost of one sort per class per state (US-019).

## Inputs

- A `ModelState` (FR-120) and a `SymmetryGroup` (FR-151): the run's group or
  an instance's stabiliser.

## Outputs

`Permutation` is owned by `quire-semantic-value`, as a type `qsl-eval`
produces and CG reads. `ModelSystem` implements FR-101's `canonical` hook and
always returns `Some((canonical state, permutation))`.

```rust
pub struct Permutation {
    // per class: the image of each key, in the class's key order
    pub maps: Vec<(DeclarationKey, Vec<(String, String)>)>,
}
```

`Permutation` acts on states (renaming references), on transition
identities (renaming the receiver and every reference argument) and on keys.
It has `compose`, `inverse` and `identity`.

## Behavior

- **Sort key.** For each class of the group and each key `k` of the class,
  the sort key SHALL be the JCS bytes of the key-free encoding of the object
  `k` names: its FR-120 state-key record (type name and fields in
  declaration order), with every reference whose universe and key lie in any
  class of the group replaced by that class's placeholder, the record
  `{"type":"symmetric","population":"<population identity text>"}`. A key
  with no object in the state SHALL sort as the empty byte string.
- **Sort.** The class's keys SHALL be ordered ascending by sort key,
  unsigned-lexicographic, with equal sort keys kept in ascending key order.
  The canonicaliser SHALL map the `i`-th key of that order to the `i`-th key
  of the class in ascending key order. The class maps together SHALL be the
  permutation `π`, and the canonical form SHALL be `π(state)`, re-encoded.
- **Identity group.** With an empty group, `canonical` SHALL return the state
  and the identity permutation.
- **Coalescing.** The engine SHALL key a stored state by its canonical
  form's FR-101 state key; two states coalesce exactly when their canonical
  forms' state-key bytes are equal.
- **Orbit membership.** The canonical form SHALL always be `π(state)` for the
  returned `π` in the group, so it is a member of the state's orbit.
- **Duplicates.** Two orbit members whose objects have equal sort keys but
  whose references point differently MAY canonicalise to different forms;
  the engine SHALL store both, with no further search.
- **Cost.** Per class of `n` keys the canonicaliser SHALL do `O(n log n)`
  comparisons of sort keys and one re-encoding of the state. It SHALL charge
  each comparison and the re-encoding to the run's meter, so reaching the
  meter stops the run (V-7).
- **Determinism.** The canonical form and the permutation SHALL be functions
  of the state and the group.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-152-AC-1 | ADR-021 §7.1, group the class `[b, c]` (stabiliser of `a`): the state `(0,1,0)` canonicalises to `(0,0,1)` with `π = (b c)`; `(0,0,1)` canonicalises to itself with the identity. Under the full class `[a, b, c]`, `(2,0,1)` canonicalises to `(0,1,2)`. | Test (TC-569) |
| FR-152-AC-2 | Exploring ADR-021 §7.1's subject under `[[a, b, c]]` stores 10 model states, one per multiset of three versions; under the stabiliser of `a` it stores 18. Every stored state is the canonical form of some reachable state, and `π(s)` equals the stored form for the returned `π`. | Test (TC-569) |
| FR-152-AC-3 | Duplicates: over a type with `v: Int[0, 1]` and `link: Option<Reference<T>>`, universe `{a, b, c}`, one class, the states `{a: (0, ->b), b: (0, ->a), c: (0, none)}` and `{a: (0, none), b: (0, ->c), c: (0, ->b)}` lie in one orbit; the canonicaliser returns a member of that orbit for each, with its permutation, and the run stores at most two states for the orbit and never fewer than one. | Test (TC-569) |
| FR-152-AC-4 | `canonical` called twice on the same state and group returns byte-equal state keys and equal permutations; with an empty group it returns the state and the identity. | Test (TC-569) |

## Dependencies

- ADR-021 SYM-7, RU-2, EI-2, EI-3.
- [FR-101](FR-101-explore-finite-models-with-canonical-order-and-the-qspec-sampler.md)
  (state key, coalescing), [FR-120](FR-120-simulate-a-checked-package-s-state-family.md)
  (state-key record and reference encoding), [FR-151](FR-151-admit-a-request-s-symmetry-declarations.md)
  (the group).
- QSpec owns the sort canonicaliser, its sort key and coalescing on
  canonical keys as normative semantics (ADR-021 QS-2); the placeholder
  record's spelling is QSpec's.

## References

- QSpec half: QSpec FR-383 (Linear STD-134; ADR-021 §9 QS-2). Owning ticket: Linear
  QSL-368.
