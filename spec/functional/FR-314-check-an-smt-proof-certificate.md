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
  - target: ix://agent-ix/quire-spec-language/FR-315
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-127
    type: depends_on
---
# FR-314: Check an SMT proof certificate

## Description

`qsl-replay`'s `check_smt_proof` SHALL accept an `SmtProofCertificate` that
comes with an SMT `Proved{BoundedComplete{depth}}` or
`Proved{Inductive{depth}}` result exactly when every query it carries is
the query FR-315 encodes for the item at that depth and every proof it
carries refutes its query using only checked Alethe rules (ADR-018 PC-6,
LA-3). FR-127's settlement map runs the check: a proof whose certificate it
accepts settles `proved` with no certification label; one it rejects
settles `inconclusive`, `CertificateRejected{rule, state}`; an SMT proof
that arrives with no certificate stays `proved`, `Uncertified`.

## Use case

A verification operator relies on a bounded-complete or inductive proof
from an SMT solver. The solver sits outside the qualified core, so the
proof loses its `uncertified` label only after the core checks the
solver's own proof objects against the queries the core itself encodes.

## Inputs

- FR-338's `CertificateRequest` for the item, and the result's basis and
  `depth`.
- An `SmtProofCertificate`.

```rust
pub enum ProofFormat { Alethe }              // cvc5's Alethe proof format

pub struct RefutedQuery {
    pub query: Vec<u8>,                       // SMT-LIB 2 script, FR-315 canonical printing
    pub proof: Vec<u8>,                       // the solver's refutation in `format`
}

pub enum SmtProofCertificate {
    BoundedComplete { format: ProofFormat, unrolling: RefutedQuery },
    Inductive { format: ProofFormat, base: RefutedQuery, step: RefutedQuery },
}

pub fn check_smt_proof(
    request: &CertificateRequest<'_>,
    basis: &ProofBasis,                       // BoundedComplete or Inductive, with depth
    certificate: &SmtProofCertificate,
) -> Result<(), CertificateRejection>;
```

FR-338's `CertificateRule` gains `QueryMismatch`, `ShapeMismatch`,
`UncheckedRule`, `ProofStepInvalid` and `NotRefutation`, and
`CertificateRejection.state` is a `CertificateLocus`:
`ProductState(ProductStateRef)` for FR-338 and FR-339, and
`Query { part }` or `ProofStep { part, index: u64 }` here, `part` being
`Unrolling`, `Base` or `Step`.

## Outputs

- `Ok(())` when the certificate is accepted.
- `CertificateRejection{rule, state}` naming the first rule that failed
  and where.

## Behavior

- The checker SHALL reject with `ShapeMismatch` at `Query { part }`, `part`
  being the first part the basis expects (`Unrolling` for
  `BoundedComplete`, `Base` for `Inductive`), a certificate whose variant
  differs from the basis: `BoundedComplete` carries one unrolling query and
  `Inductive` carries a base query and a step query.
- The checker SHALL encode each expected query with FR-315 from the
  recompiled package and the re-admitted subject at the basis's depth: the
  unrolling for `BoundedComplete{depth}`, and the base case and the step
  case for `Inductive{depth}` (ADR-018 V-2, V-3).
- The checker SHALL reject with `QueryMismatch` at that part when a
  carried query differs, byte for byte, from FR-315's canonical printing of
  the expected query.
- The checker SHALL reject with `UncheckedRule` at the first proof step
  whose rule is outside the checked Alethe rule set below, `hole` and
  `lia_generic` included.
- The checker SHALL reject with `ProofStepInvalid` at the first proof step
  whose conclusion does not follow by its checked rule from its premises,
  or whose premises are not earlier steps or assertions of its query.
- The checker SHALL reject with `NotRefutation` at the last step of a proof
  that does not conclude the empty clause.
- The checker SHALL accept a certificate only when every carried proof
  refutes its query, both the base and the step for `Inductive`.
- The checker SHALL read nothing from the solver but the certificate.

### Checked Alethe rules

The checker accepts these Alethe rules and checks each step against the
rule's definition in the Alethe specification: `assume`, `refl`, `trans`,
`cong`, `eq_reflexive`, `eq_transitive`, `eq_congruent`,
`eq_congruent_pred`, `resolution`, `th_resolution`, `contraction`,
`reordering`, `tautology`, `not_not`, `and`, `not_or`, `or`, `not_and`,
`implies`, `not_implies1`, `not_implies2`, `equiv1`, `equiv2`,
`not_equiv1`, `not_equiv2`, `ite1`, `ite2`, `not_ite1`, `not_ite2`,
`and_pos`, `and_neg`, `or_pos`, `or_neg`, `implies_pos`, `implies_neg1`,
`implies_neg2`, `equiv_pos1`, `equiv_pos2`, `equiv_neg1`, `equiv_neg2`,
`ite_pos1`, `ite_pos2`, `ite_neg1`, `ite_neg2`, `false`, `not_symm`,
`la_generic` (with its coefficients, checked by exact rational
arithmetic), `la_disequality`, `la_totality`, `la_tautology`,
`la_mult_pos`, `la_mult_neg`, `bind`, `subproof` and the `anchor`
command. Every other rule is unchecked.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-314-AC-1 | Accepted vectors: over the `Counter` subject under event-position false-extension, the TP-2 `on origin` claim `always[0,3] holds(c.value <= 3)` proved `BoundedComplete{depth: 3}`, at its horizon 3 (ADR-018 V-2), with an Alethe certificate whose unrolling query is FR-315's encoding at depth 3 and whose proof refutes it with checked rules, is accepted; `always holds(c.value <= 3)` proved `Inductive{depth: 1}` with base and step queries FR-315 encodes at depth 1, each refuted with checked rules, is accepted. Each item settles `proved`, success, with no certification label. | Test (TC-893) |
| FR-314-AC-2 | Rejected vectors: AC-1's bounded certificate with its query encoded at depth 2 is rejected `QueryMismatch` at `Unrolling`; with one step's premise replaced by a later step, `ProofStepInvalid` at that step; with one step's rule replaced by `hole`, `UncheckedRule` at that step; with its last step removed so the proof ends short of the empty clause, `NotRefutation`. AC-1's inductive certificate with an invalid step proof is rejected `ProofStepInvalid` at that `Step` step, and a `BoundedComplete` certificate offered for the `Inductive` basis is rejected `ShapeMismatch`. Each settles `inconclusive`, `CertificateRejected`. The bounded result with no certificate settles `proved`, `Uncertified`. | Test (TC-893) |

## Dependencies

- ADR-018 V-2, V-3, PC-1, PC-6, LA-2, LA-3, DS-2.
- [FR-315](FR-315-encode-the-smt-lib-transition-relation.md) (the encoding
  and its canonical printing), [FR-338](FR-338-check-an-en-1-closure-certificate.md)
  (the request, rule and rejection types),
  [FR-127](FR-127-settle-a-model-check-verdict-as-a-terminal-record.md)
  (the settlement map that runs the check).

## References

- The Alethe proof format and its rule definitions (cvc5).
- Owning ticket: Linear QSL-366; the label table: Linear QSL-390.
