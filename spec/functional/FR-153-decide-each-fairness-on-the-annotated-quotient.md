---
id: FR-153
title: "Decide each fairness on the annotated quotient"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-019
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-021
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-126
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-152
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-162
    type: depends_on
---
# FR-153: Decide each fairness on the annotated quotient

## Description

Under symmetry, a `whole` fairness constraint reads the same on the quotient
as on the subject, but an `each` constraint names one transition identity,
which a permutation maps to another. QSL's `model_check` SHALL decide weak
and strong `each` fairness on the **annotated quotient**, the retained
reduced product whose edges carry the permutation their target was
canonicalised by (ADR-021 AQ-1 to AQ-6). For each SCC it computes a cycle
group from those permutations and tests the constraint against the orbits
of that group, which is exact.

## Use case

A verification operator proves per-receiver progress, `fair weak each
attemptUpdate`, on three interchangeable configs. Symmetry still applies,
and the check rejects the cycle that never updates `a` because the cycle
group cannot carry `upd(b)` or `upd(c)` onto `upd(a)` (US-019).

## Inputs

- The retained reduced product (FR-162): edges (source, transition identity,
  canonical target, `π`), and each state's full enabled set.
- An SCC `C` from FR-126's second phase, and the clause's fairness set.

## Outputs

```rust
pub struct CycleGroup {
    pub entry: ProductStateId,                 // first state of C in discovery order
    pub tree_word: BTreeMap<ProductStateId, Permutation>,
    pub generators: Vec<Permutation>,          // non-identity closed-walk words, edge order
}
```

and, per `each` constraint, `pass` or `fail`, and for strong `each`, the
set `R` of quotient states removed for the refinement.

## Behavior

- **Spanning tree.** For an SCC `C`, the entry `c0` SHALL be its first state
  in discovery order. The engine SHALL build a breadth-first spanning tree
  of `C` from `c0` over edges inside `C` in canonical edge order (source
  discovery order, then transition identity bytes), and record for each
  node `c` the tree word `u_c`: the composition of `π⁻¹` along the tree path,
  with `u_c0` the identity.
- **Cycle group.** For each edge (`c`, `t`, `c'`, `π`) of `C`, the
  closed-walk word SHALL be `u_c ∘ π⁻¹ ∘ u_c'⁻¹`. The generators SHALL be the
  non-identity words, without repeats, in canonical edge order. The cycle
  group `H_C` is the group they generate.
- **Witness sets.** The engine SHALL compute, for an `each` constraint on
  operation `O`, `W_taken = { u_c(t) : edge (c, t, …) of C, t of O }`,
  `W_enabled = { u_c(d) : d of O enabled at c }` and
  `W_disabled = { u_c(d) : d of O disabled at c }`, over `c` in `C`, from the
  full enabled sets (ADR-019 SR-1). The identities of `O` are its FR-120
  transition identities over the subject's universes.
- **Orbit closure.** The engine SHALL close each witness set under the
  generators by breadth-first application, never by enumerating `H_C`.
- **Weak `each`.** `C` SHALL pass a weak `each` constraint on `O` exactly
  when the closures of `W_taken ∪ W_disabled` cover every identity of `O`.
- **Strong `each`.** `C` SHALL pass a strong `each` constraint on `O` (ADR-019
  SR-2 (c)) exactly when every identity in the closure of `W_enabled` is in
  the closure of `W_taken`. When it fails, with `B` the failing identities,
  the engine SHALL remove `R = { c in C : B meets u_c(E(c)) }`, recompute the
  SCCs of `C \ R`, and test each with its own cycle group, as ADR-019's
  refinement does.
- **`whole` constraints and acceptance** SHALL be read on the quotient
  unchanged (FR-126).
- **Limits.** Building trees, words, generators and closures SHALL charge the
  run's meter and count toward its time budget; reaching either SHALL stop
  the run (V-7). No count of generators, identities or refinement rounds is
  capped.
- **Determinism.** The cycle group's generators, the witness sets and the
  verdict SHALL be functions of the retained product and the fairness set.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-153-AC-1 | ADR-021 §7.1, instance `c = a`, group `{id, (b c)}`, claim `always eventually holds(c.versionNumber = 2)` under `fair weak each attemptUpdate`: the accepting SCC with `va = 0` has six canonical states and entry `(0,0,0)`; the tree word at `(0,0,1)` is `(b c)`; the non-tree edge `(0,0,0) -upd(c)-> (0,0,1)` gives generator `(b c)`; `W_taken` closes to `{upd(b), upd(c)}`, `W_disabled` is empty, and the SCC fails weak `each` because `upd(a)` is uncovered. The instance returns `Holds` over 30 product states. | Test (TC-570) |
| FR-153-AC-2 | ADR-021 §7.1's refutation: `eventually always holds(c.versionNumber != 2)` under `fair weak each attemptUpdate`, instance `c = a`: the accepting SCC spans all 18 canonical states with edges of all three labels, its cycle group is `{id, (b c)}`, the closure of `W_taken` covers `upd(a)`, `upd(b)` and `upd(c)`, and the SCC passes. | Test (TC-570) |
| FR-153-AC-3 | Strong `each`: the `Toggle` unit has object type `T` with `on: Boolean` and `fired: Boolean`, population `toggles` annotated `symmetric`, universe `{a, b}`, one class `[a, b]`, both objects initially all false, and operations `flip()` (frame `modifies self.on`, post `self.on = not pre(self.on)`) and `fire()` (frame `modifies self.fired`, pre `self.on and not self.fired`, post `self.fired`). The claim `eventually holds(forall x in toggles: x.fired)` has no `over` parameter, so it runs under the full group. Under `fair weak each flip` and `fair weak each fire` it returns `Violated`, with a loop that flips `a` and `b` alternately and never fires: each `fire` identity is disabled somewhere in that SCC. Under `fair weak each flip` and `fair strong each fire` it returns `Holds`. In that run, the SCC holding the alternating loop fails strong `each`, because both `fire` identities are enabled in it and neither is taken. The refinement removes the quotient states where a lift enables an untaken `fire`, and the SCCs left have no fair accepting cycle. | Test (TC-571) |
| FR-153-AC-4 | Every AC-1 to AC-3 run with symmetry selected gives the same verdict as the unreduced run of the same claim; repeating each run gives equal generators and equal verdicts. With the run's meter set below the cost of AC-2's orbit closure, the run stops `Stopped(ResourceExhausted, …)` naming the meter. | Test (TC-571) |

## Dependencies

- ADR-021 AQ-1 to AQ-6, AQ-8, PT-2 (`each` rows), §3 (strong fairness under
  symmetry); ADR-019 SR-1 to SR-4 for strong fairness; ADR-018 FA-4.
- [FR-126](FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md)
  (the SCC phase and the fairness filter), [FR-152](FR-152-canonicalise-a-model-state-by-sorting-its-symmetry-classes.md)
  (permutations), [FR-162](FR-162-offer-reductions-through-transition-system-hooks.md)
  (the retained edges with their permutations).
- QSpec owns `each` fairness on the annotated quotient as normative
  semantics (ADR-021 QS-3).

## References

- QSpec half: QSpec FR-384 (Linear STD-134; ADR-021 §9 QS-3). Strong fairness: ADR-019,
  Linear QSL-365. Owning ticket: Linear QSL-368.
