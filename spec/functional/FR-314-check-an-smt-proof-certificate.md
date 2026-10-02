---
id: FR-314
title: "Check an SMT proof certificate"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-015
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-018
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-338
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-127
    type: depends_on
---
# FR-314: Check an SMT proof certificate

## Description

`qsl-replay`'s `check_smt_proof` SHALL accept an `SmtProofCertificate` that
comes with an SMT `Proved{BoundedComplete{depth}}` or
`Proved{Inductive{depth}}` result exactly when its query is the query QSL
derives for the item at that depth and its proof is a valid refutation of
that query in the named proof format (ADR-018 PC-6, LA-3). A proof whose
certificate it accepts settles `proved` with no certification label; one
it rejects settles `inconclusive`, `CertificateRejected{rule, state}`; an
SMT proof that arrives with no certificate stays `proved`, `Uncertified`
(FR-127).

## Use case

A verification operator relies on a bounded-complete or inductive proof
from an SMT solver. The solver sits outside the qualified core, so the
proof loses its `uncertified` label only after the core checks the
solver's own proof object against the query the core itself derives.

## Inputs

- FR-338's `CertificateRequest` for the item, and the result's basis and
  `depth`.
- An `SmtProofCertificate`.

```rust
pub enum ProofFormat { Alethe }              // cvc5's Alethe proof format

pub struct SmtProofCertificate {
    pub format: ProofFormat,
    pub query: Vec<u8>,                       // SMT-LIB 2 script the solver refuted
    pub proof: Vec<u8>,                       // the solver's proof in `format`
}

pub fn check_smt_proof(
    request: &CertificateRequest<'_>,
    basis: &ProofBasis,                       // BoundedComplete or Inductive, with depth
    certificate: &SmtProofCertificate,
) -> Result<(), CertificateRejection>;
```

FR-338's `CertificateRule` gains `QueryMismatch`, `ProofStepInvalid` and
`NotRefutation`, and `CertificateRejection.state` is a `CertificateLocus`:
`ProductState(ProductStateRef)` for FR-338 and FR-339, `Query` or
`ProofStep { index: u64 }` here.

## Outputs

- `Ok(())` when the certificate is accepted.
- `CertificateRejection{rule, state}` naming the first rule that failed
  and where.

## Behavior

- The checker SHALL derive the item's SMT-LIB 2 query from the recompiled
  package and the re-admitted subject with `qsl-eval`'s transition-relation
  encoding: for `BoundedComplete{depth: k}` the unrolling to `k` with the
  negated property, and for `Inductive{depth: k}` the base case and the
  step case at `k`, each as ADR-018 V-2 and V-3 state them.
- The checker SHALL reject with `QueryMismatch` at `Query` when the
  certificate's query, in SMT-LIB canonical printing, differs from the
  derived query.
- The checker SHALL reject with `ProofStepInvalid` at the first Alethe
  proof step that is not an instance of an Alethe rule whose premises are
  earlier steps or assertions of the query.
- The checker SHALL reject with `NotRefutation` at the last step when the
  proof does not conclude the empty clause.
- The checker SHALL read nothing from the solver but the certificate.
- The checker SHALL accept a certificate that no rule above rejects.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-314-AC-1 | Accepted vector: over the `Counter` subject, `always holds(c.value <= 3)` proved `BoundedComplete{depth: 4}` with an Alethe certificate whose query is QSL's derived unrolling to 4 and whose proof refutes it is accepted, and the item settles `proved`, success, with no certification label. | Test (TC-893) |
| FR-314-AC-2 | Rejected vectors: AC-1's certificate with its query unrolled to 3 is rejected `QueryMismatch` at `Query`; with one proof step's premise replaced by a later step, `ProofStepInvalid` at that step; with its last step removed so the proof ends short of the empty clause, `NotRefutation`. Each settles `inconclusive`, `CertificateRejected`. The same result with no certificate settles `proved`, `Uncertified`. | Test (TC-893) |

## Dependencies

- ADR-018 V-2, V-3, PC-1, PC-6, LA-2, LA-3, DS-2.
- [FR-338](FR-338-check-an-en-1-closure-certificate.md) (the request, rule
  and rejection types), [FR-127](FR-127-settle-a-model-check-verdict-as-a-terminal-record.md)
  (the settlement that runs the check).

## References

- The Alethe proof format (cvc5).
- Owning ticket: Linear QSL-366; the label table: Linear QSL-390.
