---
id: FR-151
title: "Admit a request's symmetry declarations and compute the run's group"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-019
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-021
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-120
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-125
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-150
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-152
    type: depends_on
---
# FR-151: Admit a request's symmetry declarations and compute the run's group

## Description

A model-check request opts in to symmetry with one `SymmetryDeclaration` per
symmetric universe (ADR-021 SYM-1). QSL's `model_check`, in `qsl-analyze` (ADR-018 LA-1), SHALL check
each declaration's form against the checked package and the subject
(SYM-2), check that the subject's initial states are closed under the
declared group (SYM-4), and compute the group each `over` instance runs
under (SYM-6), all in the pre-check before any expansion. Identity
transparency is already guaranteed by S3 (FR-150), so a declaration that
passes these checks is admitted (SYM-5).

## Use case

A verification operator declares `[[a, b, c]]` for `config_history` and the
check runs one `over` instance in place of three. When one config starts at
a different version, the operator gets `SymmetryBroken` naming the initial
state and the generator, and can declare `[[a], [b, c]]` instead (US-019).

## Inputs

```rust
pub struct SymmetryDeclaration {
    pub population: DeclarationKey,
    pub classes: Vec<Vec<String>>,   // keys of the population's universe
}
```

- The `ModelSubject` (FR-125): checked package, universes, initial states.
- The checked temporal clause and its `over` parameter, or the
  deadlock-freedom item.

## Outputs

```rust
pub struct SymmetryGroup {
    pub classes: Vec<SymmetryClass>,   // every class of two or more keys
}

pub struct SymmetryClass {
    pub population: DeclarationKey,
    pub keys: Vec<String>,             // ascending identity bytes
}

pub struct InstanceGroup {
    pub representative: Option<String>, // the bound key, or none without `over`
    pub orbit: Vec<String>,             // every key the instance stands for
    pub group: SymmetryGroup,           // the run's group or the stabiliser
}
```

- A refusal, `invalid_runtime_input`/`invalid-value`, naming the
  declaration; or
- `SymmetryBroken{population, InitialStatesNotClosed{initial, generator}}`
  (FR-160); or
- the list of `InstanceGroup`s the run checks.

## Behavior

- **Form.** The pre-check SHALL refuse a declaration
  `invalid_runtime_input`/`invalid-value`, naming the declaration and the
  failing member, when: its population is not annotated `symmetric` in the
  checked package (FR-150); its population has no universe in the subject;
  a class names a key not in that universe; or a key appears in two classes
  or twice in one. The pre-check SHALL check declarations in request order
  and return the first failure.
- **Group.** The pre-check SHALL treat a key in no class, or in a class of
  one, as fixed. It SHALL form the run's group `G` as the product, over every
  declaration, of the symmetric group of each class of two or more keys, and
  hold each class's keys in ascending identity-byte order, the order FR-120's
  state key encodes them in.
- **Generators.** For each class of `m >= 2` keys `k1 < … < km`, the
  generators SHALL be the transposition `(k1 k2)` and, when `m >= 3`, the
  cycle `(k1 k2 … km)`.
- **Closed initial states.** The pre-check SHALL take each initial state in
  subject order and each generator in class order (transposition before
  cycle), apply the generator (renaming every reference whose universe and
  key it moves, in every field, collection and record, and re-encoding the
  state key) and look up the result among the initial states' keys. The
  first initial state and generator whose image is not an initial state's
  key SHALL settle the item `SymmetryBroken{population,
  InitialStatesNotClosed{initial, generator}}`, with `initial` the index in
  `subject.initial`, and no state is expanded.
- **Instances.** For a clause with an `over` parameter whose population has
  a class, the pre-check SHALL partition the universe into orbits of `G`
  (each class is one orbit; each fixed key is its own), check one instance
  per orbit, bound to the orbit's least key, and run that instance under the
  stabiliser of the bound key: `G` with the bound key removed from its
  class. A class reduced to one key by the removal SHALL drop out of the
  group. Every other instance SHALL be one `InstanceGroup` with the run's
  `G`.
- **Verdict per instance.** The verdict of an instance SHALL be the verdict
  of every key in its orbit (FR-160 records the orbit).
- **Determinism.** Admission, the group and the instance list SHALL be
  functions of the package, the subject and the declarations.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-151-AC-1 | ADR-021 §7.1: over the annotated ConfigVersion unit, universe `{a, b, c}` and one initial state with all three at version 0, `[[a, b, c]]` admits with generators `(a b)` and `(a b c)`. `ReachesTwo`'s `over` parameter gives one instance bound to `a`, orbit `[a, b, c]`, group the class `[b, c]`. | Test (TC-567) |
| FR-151-AC-2 | The same declaration over the unit without the annotation refuses `invalid_runtime_input`/`invalid-value` naming the declaration's population. `[[a, b, d]]` refuses naming `d`; `[[a, b], [b, c]]` refuses naming `b`; a declaration for a population with no universe refuses naming the population. Each refuses before any state is expanded. | Test (TC-567) |
| FR-151-AC-3 | An initial state with `a` at version 1 and `b`, `c` at 0 settles `SymmetryBroken{config_history, InitialStatesNotClosed{0, (a b)}}` under `[[a, b, c]]`, with no state expanded. Under `[[a], [b, c]]` it admits: `a` is fixed, the group is `{id, (b c)}`, and `ReachesTwo` gives two instances, `a` (group `{id, (b c)}`) and `b` (orbit `[b, c]`, group trivial). | Test (TC-568) |
| FR-151-AC-4 | Two initial states, all-zero and `(1, 0, 0)`, are not closed under `[[a, b, c]]` (`(a b)` maps the second to `(0, 1, 0)`, which is no initial state); adding `(0, 1, 0)` and `(0, 0, 1)` makes the set closed and the declaration admits. | Test (TC-568) |

## Dependencies

- ADR-021 SYM-1, SYM-2, SYM-4, SYM-5, SYM-6, EI-1 (b).
- [FR-120](FR-120-simulate-a-checked-package-s-state-family.md) (universes,
  the state key, the reference encoding), [FR-125](FR-125-read-a-model-subject-s-behaviours-as-temporal-traces.md)
  (the subject and `over` instances), [FR-150](FR-150-check-the-symmetric-population-annotation.md)
  (the annotation), [FR-152](FR-152-canonicalise-a-model-state-by-sorting-its-symmetry-classes.md)
  (the permutation action), [FR-160](FR-160-settle-reduced-verdicts-and-reduction-causes.md)
  (`SymmetryBroken`).
- QSpec owns the request member, generator closure and binding orbits as
  normative semantics (ADR-021 QS-1, QS-3).

## References

- QSpec half: QSpec FR-382 (Linear STD-134; ADR-021 §9 QS-1, QS-3). Owning ticket: Linear
  QSL-368.
