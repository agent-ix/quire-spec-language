---
id: FR-146
title: "Write per-step simulation records for a refinement"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-018
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-012
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-018
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-020
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-075
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-127
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-135
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-136
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-144
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-314
    type: depends_on
---
# FR-146: Write per-step simulation records for a refinement

## Description

QSL's S3 `requirements` hook for a refinement declaration SHALL write, beside
the refinement record (FR-144), one `operation-contract` requirement record
for the initial-state condition and one for each step row, when the
declaration has no hidden field, no history field and no `any` row (ADR-020
RE-2; QSpec FR-377 Engines and FR-377-AC-7), and SHALL offer the refinement record to the SMT unrolling
candidate as well as to the explicit-state engine (ADR-020 RE-3). QSL writes
the records and settles them; CG routes them to the SMT backend (ADR-020
DS-2) and IR encodes the obligations (DS-3). An RE-2 record never settles
`refuted`: its failure may come from an unreachable pre-state.

## Use case

A specification author wants a refinement proof that does not depend on the
universe sizes the explicit-state check needs. Each step row becomes its own
per-step simulation obligation, provable by induction over the concrete
invariants for every state of the declared types. A failing obligation says
which row could not be closed, and the explicit-state check supplies the
replayable counterexample when one exists.

## Inputs

- A `CheckedRefinement` (FR-135) with its step rows (FR-136), in the S4
  package.
- The invariant clauses whose context is an object type of the concrete
  model and each concrete operation's effective precondition, frame and
  postcondition; the abstract model's invariants, effective preconditions,
  frames and postconditions. The declaring package can hold the abstract
  model as well as the concrete one (ADR-020 RM-1), so an invariant of the
  package is a concrete invariant only when its context is a concrete-model
  object type.

## Outputs

```rust
pub struct StepSimulationRecord {
    pub refinement: NodeKey,
    pub obligation: StepObligation,
    pub hypotheses: Vec<NodeKey>,      // invariants whose context is a concrete-model object type
}

pub enum StepObligation {
    Initial,                           // ADR-020 RS-2
    Step { row: StepRowRef },          // RS-3 for `stutter`, RS-4 for an abstract operation;
                                       // the row's zero-based position in the refinement node's `steps` order
}
```

Each is written as a requirement record of kind `operation-contract`. The
record's occurrence key and obligation identity are not yet specified: see
"Record identity, pending" below.
`StepSimulationRecord` and `StepObligation` live in `quire-semantic-value`,
since the compiler writes them and CG reads them (ADR-018 LA-5).

## Behavior

### Which records

- When the refinement has no hidden field, no history field and no `any`
  row, the `requirements` hook SHALL write one `Initial` record and one
  `Step` record for each step row.
- When the refinement has a hidden field, a history field or an `any` row,
  the hook SHALL write no `StepSimulationRecord`: an `any` row has no
  per-step obligation, so the other rows' records would not discharge the
  safety half, and with history fields `map` is not a function of
  `(s, s')`.

### What each record states

- A `Step` record for a row of concrete operation `o` SHALL state: for every
  pair `(s, s')` of states of the declared types, if the concrete invariants
  hold at `s`, `o`'s effective precondition holds at `s`, `(s, s')` is
  within `o`'s frame and `o`'s postcondition holds over `(s, s')`, then
  RS-3 (for `stutter`) or RS-4 (for an abstract operation) holds over
  `(map(s), map(s'))`, with the row's receiver and arguments evaluated at
  `s` and `_` arguments existentially quantified over their parameter
  domains.
- The `Initial` record SHALL state that `map(c0)` is an abstract initial
  state for every concrete initial state `c0`.
- `hypotheses` SHALL list every invariant clause whose context is an object
  type of the concrete model, each of which is its own `operation-contract`
  record, ascending by node-key digest with no duplicate.
- An invariant clause of the declaring package whose context is an object
  type of the abstract model only SHALL NOT be listed in `hypotheses`,
  whether the abstract model comes from the declaring package or from a
  dependency.

### Row order

- The `StepRowRef` of a `Step` record SHALL be the zero-based position of
  its row in the refinement node's `steps` array, which QSpec FR-375 sorts
  ascending by the node-key digest of the row's first named declaration (a
  model subject's row names its concrete operation), then by member name as
  UTF-8 bytes. A model subject has one row per concrete operation, so no two
  rows tie.
- Reordering the `step` rows in source SHALL change no record's `StepRowRef`.
- The checker SHALL give a protocol subject's `Step` record the `StepRowRef`
  that is the row's zero-based position in FR-136's protocol sort order, which
  is QSpec FR-375's: the node-key digest of the declaration the row names (the
  selected clause for an `activate` row), a row that names no declaration (a
  `memory` row) after every row that names one, and rows that tie ordered by
  the UTF-8 bytes of the alias-free class spelling (the memory kind name for
  `memory` rows). Reordering the rows in source SHALL change none.

### Record identity, pending

The requirement record's occurrence key (ADR-013 O-07) and obligation
identity (ADR-013 O-09) for these records are pending an ADR-013 O-07/O-09
amendment, a CG ask for the digest, and a QSpec FR-322 occurrence-role
ruling. This requirement states no role, ordinal or identity preimage. The
types `StepRowRef`, `StepObligation` and the `hypotheses` rule are owned by
quire-semantic-value FR-123, which still says "declaration order" and "the
concrete package's invariant clauses"; it is to change to this requirement's
wording.

### Routing and settlement

- The records SHALL be offered to negotiation like any `operation-contract`
  record (FR-075); a record no candidate settles SHALL settle V-8
  `unsupported`.
- The refinement record (FR-144) SHALL also be a candidate for SMT unrolling
  (ADR-018 EN-2) with the `check_step` edge monitor (ADR-020 RE-3); a
  refutation from it SHALL settle `refuted` only through FR-145's replay.
- QSL SHALL settle an RE-2 record's backend outcome as V-3 `Proved{basis:
  Inductive{depth: 1}, certification}` with basis `decisive-witness` when the
  obligation holds, `Certified` when FR-314 verifies the backend's proof
  certificate and `Uncertified` when it is absent or FR-314 cannot verify it
  without showing it wrong (ADR-018 PC-6), and as V-6 `Inconclusive(InductionNotClosed{depth: 1})` when it
  fails from a pre-state that satisfies the hypotheses. A backend
  counterexample for an RE-2 record SHALL settle
  `Inconclusive(InductionNotClosed{depth: 1})` and never `refuted`.
- A backend outcome "holds" whose proof certificate FR-314 shows wrong
  SHALL settle V-6 `Inconclusive(CertificateRejected{rule, at})` with
  FR-314's rule and locus (ADR-018 PC-2), never `proved`.
- A certificate FR-314 cannot verify without showing it wrong SHALL settle
  `Uncertified`, not `CertificateRejected`. QSL SHALL settle every RE-2
  record of a refinement `Certified` exactly when `check_refinement_smt_proof`
  (FR-314 "Refinement certificates") returns `Verified` for the refinement's
  certificate, against the queries FR-315 encodes (FR-315 "Refinement
  queries"); `Uncertified` when it returns `Unverifiable` or no certificate
  came; and, when it rejects at a part, the record of that part (the `Initial`
  record for `Base`, row `r`'s `Step` record for `Step { row: Some(r) }`)
  `Inconclusive(CertificateRejected{rule, at})` and every other record
  `Uncertified`. A refinement with a row whose query FR-315 refuses to encode (a
  `_` argument, an `any` row or a term outside `QF_LIA`) gets `Unverifiable`
  from the checker (FR-314), so every record of it is `Uncertified` whatever
  certificate arrives; a backend counterexample still settles
  `Inconclusive(InductionNotClosed{depth: 1})`.
- QSL SHALL settle the refinement's safety half from its records and the
  hypothesis items by QSpec FR-468 "Settlement", in its order: an initial
  counterexample, a hypothesis item not `proved`
  (`hypotheses-not-established`), a step counterexample, a certificate FR-314
  shows wrong (`certificate-rejected{rule, at}`), then the verified or
  unverified proofs, the half being `certified` only when every part and every
  hypothesis item is certified, with other backend results as FR-468 states.
  The certificate FR-314 checks for it is a `RefinementSmtCertificate`
  (FR-314 "Refinement certificates"): one base part and one step part per row
  in `steps` order.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-146-AC-1 | ADR-020 §8's `CasRefinesCounter` writes eight `operation-contract` records beside its refinement record: one `Initial` and one `Step` for each of its seven rows, each listing the invariant clauses whose context is a concrete-model object type (none) as hypotheses. With the `commitB` row changed to `-> any` it writes none. | Test (TC-551) |
| FR-146-AC-2 | `RegisterHistory` (FR-138) and `Coin` with `side` hidden (FR-141-AC-4) write no `StepSimulationRecord`; each still writes its refinement record. | Test (TC-551) |
| FR-146-AC-3 | With an invariant `CasInv` (`self.busyA or self.tmpA <= self.value`) added to `Impl::Counter`, every `StepSimulationRecord` of `CasRefinesCounter` lists `CasInv` in `hypotheses`, the request also holds `CasInv`'s own `operation-contract` record. | Test (TC-551) |
| FR-146-AC-4 | Settlement: a backend outcome "holds" for `CasRefinesCounter`'s `commitA` record settles `proved`, `decisive-witness`, `Proved{Inductive{depth: 1}, Uncertified}` with no certificate and `Proved{Inductive{depth: 1}, Certified}` for every record of a refinement whose certificate `check_refinement_smt_proof` verifies, a refinement with a `_` row having every record `Uncertified` ; for the lost-update model, a backend counterexample for the `commitB` record from `(1, 0, f, 0, t)` settles `inconclusive`, `Inconclusive(InductionNotClosed{depth: 1})`; with no SMT candidate registered, each record settles `unsupported`. | Test (TC-551) |
| FR-146-AC-5 | Over a model subject, `StepRowRef` is the zero-based position in the `steps` order QSpec FR-375 fixes (node-key digest of the first named declaration, then member name). For `CasRefinesCounter`, the seven `Step` records name the positions 0 to 6, each once, and the row at position i has a first named declaration whose node-key digest is not greater than the row at position i + 1's; writing the seven `step` rows in reverse source order gives each `Step` record the same `StepRowRef`. Over FR-136-AC-7's protocol (with its `unknown`-delivery channel and `tso` `parallel`) the `Step` records name the positions of FR-375's protocol order (a `memory` row after every declaration-naming row, ties by alias-free class spelling), and reversing the source order of the rows changes none. | Test (TC-551) |
| FR-146-AC-6 | With a package that holds `Spec::Counter` and `Impl::Counter` and an invariant `AbsInv` on `Spec::Counter` beside `CasInv` on `Impl::Counter`, every `StepSimulationRecord` of `CasRefinesCounter` lists `CasInv` and not `AbsInv` in `hypotheses`, and writing the clauses in the opposite source order leaves each record's `hypotheses` unchanged. | Test (TC-551) |
| FR-146-AC-7 | The settlement map treats a certificate by what FR-314 finds: a backend outcome "holds" for a `Step` record whose certificate FR-314 shows wrong settles `inconclusive`, `unsettled`, `Inconclusive(CertificateRejected{rule, at})` with FR-314's rule and locus, never `proved`; the same outcome with a certificate that FR-314 cannot verify and does not show wrong settles `Proved{Inductive{depth: 1}, Uncertified}`. The vector feeds the settlement map the checker's `Rejected` and `Unverifiable` results directly. | Test (TC-551) |
| FR-146-AC-8 | The safety half of `CasRefinesCounter` settles from its eight records and hypothesis items by QSpec FR-468: `proved`, `Certified` when all eight records are proved with verified certificates and every hypothesis item is certified; `proved`, `Uncertified` when one hypothesis item is `Uncertified`; `inconclusive`, `hypotheses-not-established` for the invariant `self.value < 0` whose hypothesis items are not proved, although every step record is proved; `Inconclusive(InductionNotClosed{depth: 1})` for a backend counterexample for one `Step` record; `refuted` through FR-145's replay for a counterexample for the `Initial` record. | Test (TC-551) |

## Dependencies

- ADR-020 §5 RE-2, RE-3, "Claim kinds" and "Strength", §9 DS-2 and DS-3;
  ADR-018 EN-2 and V-3, V-6, V-8; ADR-012 (requirement records at S3);
  ADR-013 O-07 and O-09 (identity pending); ADR-018 PC-2 and PC-6; ADR-020
  RM-1; QSpec FR-375 (row order).
- [FR-075](FR-075-compute-candidates-from-registered-backends.md),
  [FR-127](FR-127-settle-a-model-check-verdict-as-a-terminal-record.md),
  [FR-135](FR-135-check-a-refinement-declaration-s-subjects-and-state-mapping.md),
  [FR-136](FR-136-check-a-refinement-s-step-rows.md),
  [FR-314](FR-314-check-an-smt-proof-certificate.md),
  [FR-144](FR-144-request-and-settle-a-refinement-item.md).

## References

- The QSpec half (the per-step records as QSpec FR-290's
  `operation-contract` row): QSpec FR-377 and FR-379 (Linear STD-133).
