---
id: FR-338
title: "Check an EN-1 closure certificate"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-015
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-018
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-120
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-125
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-126
    type: depends_on
---
# FR-338: Check an EN-1 closure certificate

## Description

`qsl-replay`'s `check_closure` SHALL accept a `ClosureCertificate` that EN-1
returns with a safety proof exactly when the certificate's product states
contain every initial product state, are closed under expansion, and hold
no bad state, recomputing every successor with the core's own code
(ADR-018 PC-3, PC-5, LA-3). A proof whose certificate it accepts settles
`proved`, `Certified` (FR-127); one it rejects settles `inconclusive`,
`CertificateRejected`.

## Use case

A verification operator relies on a model-check proof of an invariant. The
engine that searched the state space lives outside the qualified core, so
the proof counts only after the core recomputes the closure the engine
claims and finds every successor inside it and no bad state.

## Inputs

- A `CertificateRequest`: the checked package recompiled by FR-098's rules,
  the model subject re-admitted by FR-120 from its byte provision and
  universes (FR-125), and the item: a TP-1, TP-2 or TP-3 clause, or the
  deadlock-freedom item (FR-124).
- A `ClosureCertificate`.

```rust
pub struct ProductStateRef {
    pub model_state: DigestRecord,          // quire.simulation.state-key/v1
    pub automaton_state: AutomatonStateKey, // the translation's canonical key
}
pub struct ClosureCertificate { pub states: Vec<ProductStateRef> }

pub enum CertificateRule {
    InitialMissing, SuccessorMissing, BadState, NotPartition, BackwardEdge,
    WitnessFails, QueryMismatch, ShapeMismatch,
    ProofStepInvalid, NotRefutation,
}
pub struct CertificateRejection { pub rule: CertificateRule, pub state: CertificateLocus }
pub enum CertificateLocus {
    ProductState(ProductStateRef),
    Query { part: QueryPart },
    ProofStep { part: QueryPart, index: u64 },
}
pub enum QueryPart { Unrolling, Base, Step }   // FR-314

pub fn check_closure(request: &CertificateRequest<'_>, certificate: &ClosureCertificate)
    -> Result<(), CertificateRejection>;
```

`AutomatonStateKey` is the property-automaton translation's canonical
encoding of an automaton state (`qsl-eval`, ADR-018 LA-2), the same for
every order in which the states are materialized.

## Outputs

- `Ok(())` when the certificate is accepted.
- `CertificateRejection{rule, state}` naming the first rule that failed
  and the product state it failed at.

## Behavior

- The checker SHALL rebuild the item's property automaton with
  `qsl-eval`'s translation, and SHALL read nothing from the engine but the
  certificate.
- The checker SHALL reject with `InitialMissing` when an initial product
  state is not in the certificate.
- Starting from the initial product states, the checker SHALL expand each
  reached state through `ModelSystem`, the automaton and the terminal
  rules (FR-125), and SHALL reject with `SuccessorMissing` when a successor
  product state is not in the certificate.
- The checker SHALL reject with `BadState` when a reached state's monitor
  state rejects (a bounded-profile closure at a terminal state included),
  its letter is undefined, or its expansion records `ContractUndetermined`;
  for the deadlock-freedom item it SHALL also reject with `BadState` a
  deadlocked state (FR-124). A deadlocked state is bad for no other item:
  it reads by the terminal rules (ADR-018 DL-6).
- The checker SHALL expand no state outside the certificate, so its work is
  bounded by the certificate's size, and SHALL accept when every reached
  state is expanded.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-338-AC-1 | Accepted vector: over ADR-018 §6's subject, FR-126-AC-2's TP-1 claim `always holds(c.versionNumber <= 1000)` returns `Holds` with a `ClosureCertificate` of its explored product states, and `check_closure` accepts it. | Test (TC-525) |
| FR-338-AC-2 | Rejected vectors: AC-1's certificate with the product state for model state `(1, 0)` removed is rejected `SuccessorMissing` at that state; with the initial product state removed, `InitialMissing` at it; over the `Counter` subject with no `terminal` member, a certificate holding every reachable state, offered for the deadlock-freedom item, is rejected `BadState` at value 3 (deadlocked), while the same certificate for `always holds(c.value <= 3)` is accepted, since the deadlock at 3 changes no authored claim (ADR-018 DL-6). | Test (TC-525) |
| FR-338-AC-3 | Over FR-120-AC-9's `test/tallies` subject, a certificate holding `t1` for `always holds(true)` is rejected `BadState` at `t1`, whose expansion records `ContractUndetermined` for the undefined `pre Low`. | Test (TC-525) |

## Dependencies

- ADR-018 PC-3, PC-5, LA-2, LA-3.
- [FR-120](FR-120-simulate-a-checked-package-s-state-family.md),
  [FR-125](FR-125-read-a-model-subject-s-behaviours-as-temporal-traces.md),
  [FR-126](FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md)
  (the certificate's producer), [FR-127](FR-127-settle-a-model-check-verdict-as-a-terminal-record.md)
  (the settlement that runs the check).

## References

- Owning ticket: Linear QSL-366.
