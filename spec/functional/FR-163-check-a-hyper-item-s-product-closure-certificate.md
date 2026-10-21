---
id: FR-163
title: "Check a hyper item's product-closure certificate in the qualified core"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-021
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-018
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-023
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-098
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-120
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-127
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-174
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-176
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-177
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-178
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-338
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-179
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-181
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-182
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-331
    type: references
---
# FR-163: Check a hyper item's product-closure certificate in the qualified core

## Description

When EN-1 returns `Holds` for an HP-1, HP-2, HP-3, HP-5 or HP-6 item, the
layer-A engine in `qsl-analyze` SHALL hand over a
`ProductClosureCertificate`, and QSL's layer-6 crate `qsl-replay` SHALL hold
`check_product_closure`, which accepts the certificate only when the
certified set holds every initial product state, is closed under the
recomputed successors, holds no undefined, refused or incomplete letter or
match, and holds no violation by the form's rule (ADR-023 HX-7). The
certificate reuses ADR-018's closure certificate (PC-3) and component
certificate (PC-4). The checker recompiles the package and reads nothing
from the engine's search but the certificate, so a certified hyper `proved`
does not depend on the engine (ADR-029 CB-2, RU-2).

## Use case

A security lead wants a noninterference proof that counts as qualified
evidence. The model checker proves the secure vault leaks nothing and hands
over the product states it explored with their component witnesses. A small
checker in the qualified core rebuilds the self-composed product from the
model, confirms the set is closed and holds no fair accepting cycle, and
accepts it, so the proof stands without trusting the model checker's
search.

## Inputs

- FR-098's request, with each alias's subject: initial snapshots in the
  byte provision and the alias's universes; the request's
  `ModelCheckLimits`.
- The certificate, serialized with RFC 8785 JCS:

```rust
pub struct ProductClosureCertificate {
    pub form: HyperForm,                   // HP-1, HP-2, HP-3, HP-5 or HP-6
    pub instance: Vec<(Name, Key)>,        // object parameter binding
    pub reduction: Option<CopySwap>,       // when FR-174 applied copy-swap
    pub closure: ClosureCertificate,       // ADR-018 PC-3: product state keys,
                                           // sorted ascending, no duplicates;
                                           // for HP-1, each subject's state keys
    pub components: Option<Vec<Component>>, // ADR-018 PC-4, for HP-2 and HP-3
}
```

A product state key is the tuple of each component's
`quire.simulation.state-key/v1` digest in variable order, with the property
automaton's canonical state key (ADR-018 PC-3), and for HP-3 the witness
set's members as sorted (existential tuple key, automaton key) pairs.

For HP-3, `states` is the universal tuple and each witness pair's tuple is
the existential tuple, each in its own quantifier order (ADR-023 HC-2).
When there are no universal variables, the universal tuple is empty. An
empty tuple and an absent tuple are different values. The witness set may
also be empty; neither case introduces a component state or an automaton
state that the product does not contain.

## Outputs

```rust
pub enum ProductClosureCheck {
    Accepted,
    Rejected(CertificateRejection),          // FR-338's: rule and locus
    Stopped(ReachedLimit),                   // FR-126: the limit and its value
}
```

The checker uses FR-338's `CertificateRejection{rule, at}`; `Rejected`
settles ADR-018 PC-2's `CertificateRejected{rule, at}`. It reuses
FR-338's `InitialMissing` (a missing initial product state),
`SuccessorMissing` (a missing successor), `NotPartition` (components that
do not partition the members), `BackwardEdge` (an edge from a component to
an earlier one) and `WitnessFails` (a component whose witness fails), and FR-338's `CertificateRule` gains
`Malformed`, `UndefinedMember` and `ViolatingMember`. `CertificateLocus`
gains `HyperProductState(ProductStateKey)`, the hyper product's state key,
`HyperTransitionTuple`, the ordered tuple of labelled edges of an HP-1
check, and `CertificateMember`, the name of a malformed certificate member.
An HP-1 edge retains its pre-state key, transition identity (operation,
receiver and arguments), post-state key and observed result as FR-179
retains them. The tuple is in execution-variable binding order. It is not
the tuple of source states: two edges from the same source state can have
different arguments, post-states or results. QSpec FR-331 owns their wire
spelling.

## Behavior

- **Production.** When EN-1 returns `Holds` for one of these forms, the
  `qsl-analyze` engine SHALL write the certificate from its retained
  product: every stored product state, and for HP-2 and HP-3 the
  components of its SCC decomposition, each with the first PC-4 witness
  that holds for it.
- **Core code only.** `check_product_closure` SHALL build each alias's
  `ModelSystem` by FR-128's rules and SHALL expand product states through
  `qsl-eval`'s hyper product successor function, property-automaton
  translation, witness-set step and fairness-enabledness predicates (ADR-018
  LA-2), using no `qsl-analyze` code.
- **Malformed.** A certificate whose `closure` keys are unsorted or
  duplicated, or whose `components` are absent for HP-2 or HP-3, SHALL be
  rejected with rule `Malformed` before any recomputation. Absent
  `components` SHALL name `CertificateMember("components")`, also when
  `closure` is empty. An unsorted or duplicate key SHALL name the first
  offending key in the supplied closure sequence. The checker SHALL NOT
  substitute a made-up member key for an absent structure.
- **Initial.** Every recomputed initial product state SHALL be a member;
  otherwise the checker SHALL reject with rule `InitialMissing`.
- **Closure.** Every recomputed successor of every member, canonicalised by
  FR-174's copy-swap rule when `reduction` is set, SHALL be a member;
  otherwise the checker SHALL reject with rule `SuccessorMissing`, naming
  the member.
- **Undefined.** No member's body letters, `μ_U` or `μ_E` SHALL evaluate
  undefined, refused or incomplete; otherwise the checker SHALL reject with
  rule `UndefinedMember`.
- **Components (HP-2, HP-3).** The checker SHALL check PC-4 (b) to (d) over
  the members, in this order: the components partition them, or the
  checker SHALL reject with rule `NotPartition`; every recomputed edge goes
  from a component to itself or to a later one, or it SHALL reject with
  rule `BackwardEdge`; each component's witness holds, or it SHALL reject
  with rule `WitnessFails`.
- **Violation by form.** The checker SHALL reject with rule
  `ViolatingMember` when the
  closed set holds:
  - HP-2: a component whose PC-4 witness does not exclude a fair accepting
    cycle of the self-composition;
  - HP-3: a member whose witness set is empty and whose component is not
    `Trivial` or `UnfairWeak` under the universal variables' fairness sets;
  - HP-6: a member whose monitor state rejects (PC-3 (c));
  - HP-1: a tuple of member transitions, one per execution variable with
    its step label, on which the body evaluates false.
  For HP-1, `ViolatingMember` SHALL name the complete
  `HyperTransitionTuple`, in ADR-023 HC-3 / FR-179's canonical edge-tuple
  order. An `UndefinedMember` caused by evaluating that tuple's body SHALL
  name the same tuple. Other failures on an HP-1 subject's state SHALL name
  the actual state member, without an invented automaton key.
  For HP-5 the closure, initial and undefined rules are the whole check,
  since its witnesses are checked by their own replay (FR-181).
- **Order.** The checker SHALL apply the rules in the order listed and
  report the first failing rule with the locus specified above: the absent
  certificate member, offending state key or labelled transition tuple.
- **Limits.** The checker SHALL count recomputed states and successors
  against the request's `ModelCheckLimits` with checked arithmetic.
- **Stop.** When a count reaches its limit, the checker SHALL return
  `Stopped`, naming the limit and its value, which settles V-7,
  `incomplete`, `LimitReached{limit, value, setting}`.
- **Purity.** The checker SHALL read no path, environment variable, clock
  or search location, and its result SHALL be a function of the
  certificate, the request and the limits.
- **Settlement.** FR-182 SHALL settle `proved`, `Certified` only on
  `Accepted`, V-6 `CertificateRejected{rule, at}` on `Rejected`, and V-7
  on `Stopped`.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-163-AC-1 | Accepted vectors: §8.1's secure `NonInterference` certificate (8 product states, every component `MissingAcceptance` or `Trivial`) is `Accepted`; the copy-swap certificate (6 states) is `Accepted`; §8.2's secure `Opaque` certificate is `Accepted`; FR-179-AC-1's `Det` certificate is `Accepted`; FR-181-AC-1's HP-5 certificate is `Accepted`. | Test (TC-645) |
| FR-163-AC-2 | Rejected vectors over the AC-1 `NonInterference` certificate: one member removed is `Rejected` with rule `SuccessorMissing` naming the member whose successor is missing; its keys unsorted is `Rejected` with rule `Malformed` before any recomputation; an added member whose `l` components differ, in a component marked `MissingAcceptance`, is `Rejected` with rule `WitnessFails`; one member listed in two components is `Rejected` with rule `NotPartition`; two components joined by an edge listed in the reverse order is `Rejected` with rule `BackwardEdge`; the initial product state removed is `Rejected` with rule `InitialMissing`. | Test (TC-646) |
| FR-163-AC-3 | A certificate for the leaky vault's `NonInterference` built by listing its 14 product states with the components of the secure run, which do not cover the leaky states, is `Rejected` with rule `NotPartition`, so a false proof is never accepted; a certificate for FR-181-AC-5's body listing the reached product states is `Rejected` with rule `UndefinedMember` at the first state with `l = 1`. | Test (TC-646) |
| FR-163-AC-4 | The AC-1 `NonInterference` certificate checked with `max_states` 1 returns `Stopped` with `{MaxStates, 1}` naming the limit and its value; checking it twice gives equal results. | Test (TC-646) |
| FR-163-AC-5 | HP-2 and HP-3 certificates with absent `components`, including one with an empty closure, reject `Malformed` at `CertificateMember("components")` before any successor recomputation, without selecting a state key. | Test (TC-646) |
| FR-163-AC-6 | An HP-1 certificate for FR-179-AC-2's nondeterministic `Det` subject rejects `ViolatingMember` at the first complete labelled edge tuple in canonical order; equal pre-state keys and arguments with different post-states remain distinct. FR-179-AC-4's undefined body rejects `UndefinedMember` at its complete edge tuple. | Test (TC-646) |
| FR-163-AC-7 | For ADR-023 HC-2 with zero universal variables, a supplied key with an empty universal tuple and its actual witness set retains both on serialization and reading; the empty witness set remains distinct from an absent witness set. No component is added to satisfy a nonempty-array constraint. | Test (TC-646) |

## Dependencies

- ADR-023 HX-7 and HV-1; ADR-018 PC-1 to PC-5 and LA-2; ADR-029 CB-2 and
  RU-2.
- [FR-098](FR-098-execute-a-replay-request.md) (recompile),
  [FR-120](FR-120-simulate-a-checked-package-s-state-family.md)
  (`ModelSystem`), [FR-127](FR-127-settle-a-model-check-verdict-as-a-terminal-record.md)
  (PC-3 and PC-4 certificates),
  [FR-174](FR-174-reduce-hyper-products-and-check-copy-swap-symmetry.md)
  (copy-swap canonicalisation).
- FR-176 to FR-179 and FR-181 produce the certificate; FR-182 settles it.

## References

- ADR-023. QSpec half: QSpec FR-399 (Linear STD-136; ADR-023 QS-5). The
  certified-proof ruling is recorded on Linear QSL-390.
