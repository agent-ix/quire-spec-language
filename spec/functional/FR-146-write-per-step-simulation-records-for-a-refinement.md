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
  - target: ix://agent-ix/quire-spec-language/FR-095
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

Each is written as a requirement record of kind `operation-contract`, keyed
by an occurrence key and identified by an obligation identity, both defined
under "Record identity" below.
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
- Each record's obligation identity SHALL bind the refinement node, the
  obligation and the hypotheses (ADR-013 O-09), as "Record identity"
  states.

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
  record, ascending by node-key digest with no duplicate. An invariant
  clause of the declaring package whose context is an object type of the
  abstract model only SHALL NOT be listed, whether the abstract model comes
  from the declaring package or from a dependency.

### Record identity

- **Row order.** A `Step` record's `StepRowRef` SHALL be the zero-based
  position of its row in the refinement node's `steps` array, which QSpec
  FR-375 sorts ascending by the node-key digest of the row's first named
  declaration, then by member name as UTF-8 bytes. It is not the row's
  position in source: reordering the `step` rows in source SHALL change no
  record's `StepRowRef`, occurrence key or obligation identity. FR-136
  states the order of `CheckedRefinement.steps`.
- **Occurrence key.** Each record's occurrence key (ADR-013 O-07) SHALL
  have the refinement node's id as its node id and the role of the
  refinement record's own occurrence key (FR-144), whose ordinal 0 it
  keeps. The `Initial` record's ordinal SHALL be 1 and a `Step` record's
  SHALL be 2 plus its `StepRowRef`. The package source map (FR-095) SHALL
  map the `Initial` key to the regions of the refinement declaration and a
  `Step` key to the regions of its `step` row, so a counterexample or
  diagnostic for a record resolves to the source that caused it.
- **Obligation identity.** A record's obligation identity SHALL be ADR-013
  O-09's digest of the RFC 8785 encoding of: the refinement node's id, the
  record's occurrence key, the CG `ObligationKind` requested, an empty
  `arguments` list (a step obligation quantifies over every state of the
  declared types, not over argument domains), the record's `StepObligation`
  and its `hypotheses`. The preimage names the `StepObligation` and the
  `hypotheses` because the occurrence key alone does not carry the
  hypotheses. This subject kind needs no separate tag: the refinement
  node's id never equals a clause, application or function node id.

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
  FR-314's rule and locus (ADR-018 PC-2) and never `proved`, whatever the
  backend said.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-146-AC-1 | ADR-020 §8's `CasRefinesCounter` writes eight `operation-contract` records beside its refinement record: one `Initial` and one `Step` for each of its seven rows, each listing the concrete package's invariant clauses (none) as hypotheses. With the `commitB` row changed to `-> any` it writes none. | Test (TC-551) |
| FR-146-AC-2 | `RegisterHistory` (FR-138) and `Coin` with `side` hidden (FR-141-AC-4) write no `StepSimulationRecord`; each still writes its refinement record. | Test (TC-551) |
| FR-146-AC-3 | With an invariant `CasInv` (`self.busyA or self.tmpA <= self.value`) added to `Impl::Counter`, every `StepSimulationRecord` of `CasRefinesCounter` lists `CasInv` in `hypotheses`, the request also holds `CasInv`'s own `operation-contract` record, and the `Initial` record's obligation identity differs from the one without `CasInv`. | Test (TC-551) |
| FR-146-AC-4 | Settlement: a backend outcome "holds" for `CasRefinesCounter`'s `commitA` record settles `proved`, `decisive-witness`, `Proved{Inductive{depth: 1}, Uncertified}` with no certificate and `Proved{Inductive{depth: 1}, Certified}` with a certificate FR-314 verifies; for the lost-update model, a backend counterexample for the `commitB` record from `(1, 0, f, 0, t)` settles `inconclusive`, `Inconclusive(InductionNotClosed{depth: 1})`; with no SMT candidate registered, each record settles `unsupported`. | Test (TC-551) |
| FR-146-AC-5 | `StepRowRef` is the position in the digest-sorted `steps` order. For `CasRefinesCounter`, the seven `Step` records name the positions 0 to 6, each once, and the row at position i has a first named declaration whose node-key digest is not greater than the row at position i + 1's; writing the seven `step` rows in the reverse source order writes the same eight records, each with the same `StepRowRef`, occurrence key and obligation identity. | Test (TC-551) |
| FR-146-AC-6 | With a package that holds `Spec::Counter` and `Impl::Counter` and an invariant `AbsInv` on `Spec::Counter` beside `CasInv` on `Impl::Counter`, every `StepSimulationRecord` of `CasRefinesCounter` lists `CasInv` and not `AbsInv` in `hypotheses`, and writing the clauses in the opposite source order leaves each record's `hypotheses` and obligation identity unchanged. | Test (TC-551) |
| FR-146-AC-7 | `CasRefinesCounter`'s eight records and its refinement record have nine distinct occurrence keys with the refinement node's id and one role; the refinement record keeps ordinal 0, `Initial` has ordinal 1 and the `Step` record at `StepRowRef` r has ordinal 2 + r. The source map maps `Initial`'s key to the declaration's regions and a `Step` key to its row's regions. Two obligation identities differ when only the `StepObligation` differs and when only the `hypotheses` differ (FR-146-AC-3), and are equal when neither differs and only the source order of rows or clauses does. | Test (TC-551) |
| FR-146-AC-8 | A backend outcome "holds" for `CasRefinesCounter`'s `commitA` record with an Alethe certificate whose step proof has one premise replaced by a later step (FR-314-AC-2's vector) settles `inconclusive`, `unsettled`, `Inconclusive(CertificateRejected{ProofStepInvalid, at})` with `at` the `ProofStep` locus FR-314 names, never `proved`; the same outcome with a certificate that uses `hole` and is not shown wrong settles `Proved{Inductive{depth: 1}, Uncertified}`. | Test (TC-551) |

## Dependencies

- ADR-020 §5 RE-2, RE-3, "Claim kinds" and "Strength", §9 DS-2 and DS-3;
  ADR-018 EN-2 and V-3, V-6, V-8; ADR-012 (requirement records at S3);
  ADR-013 O-09.
- [FR-075](FR-075-compute-candidates-from-registered-backends.md),
  [FR-127](FR-127-settle-a-model-check-verdict-as-a-terminal-record.md),
  [FR-135](FR-135-check-a-refinement-declaration-s-subjects-and-state-mapping.md),
  [FR-136](FR-136-check-a-refinement-s-step-rows.md),
  [FR-095](FR-095-occurrence-keyed-source-map-and-locus.md),
  [FR-314](FR-314-check-an-smt-proof-certificate.md),
  [FR-144](FR-144-request-and-settle-a-refinement-item.md).

## References

- The QSpec half (the per-step records as QSpec FR-290's
  `operation-contract` row): QSpec FR-377 and FR-379 (Linear STD-133).
