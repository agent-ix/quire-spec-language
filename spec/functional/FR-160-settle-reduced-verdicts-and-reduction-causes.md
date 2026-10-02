---
id: FR-160
title: "Settle reduced verdicts and reduction causes as terminal records"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-019
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-021
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-069
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-126
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-127
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-159
    type: depends_on
---
# FR-160: Settle reduced verdicts and reduction causes as terminal records

## Description

A run that completes with symmetry or partial-order reduction applied and
finds no counterexample is a proof by its own technique,
`ProofBasis::Reduced`, never `Exhaustive` (ADR-021 RV-1). No core checker
verifies a reduced proof's certificate, so it is labelled
`Certification::Uncertified` (ADR-018 PC-1, RU-6). QSL SHALL add
`Reduced` and four inconclusive causes, `ReductionNotPreserving`,
`SymmetryBroken`, `ConstraintReached` and `ReductionHorizon`, to FR-127's
terminal values and settle each through FR-127's one map (RV-2 to RV-5).
`max_depth` is the search horizon, never a limit: a partial-order run that
completes to it settles `ReductionHorizon` (RV-5). The reduced and unreduced
runs of every corpus model settle the same verdict (RV-10). A refutation is concrete and
unreduced whatever found it (RV-7).

## Use case

A verification operator counts proofs. A symmetry-reduced proof says so,
names the group and the bounds it rests on, and is never counted as an
exhaustive proof; a refutation says nothing about how it was found, because
its counterexample replays without the reduction (US-019).

## Inputs

- A `ModelCheckOutcome` from a run with reductions (FR-126, FR-159), the
  admitted reductions and their table rows, and each instance's orbit
  (FR-151).

## Outputs

```rust
pub enum ProofBasis {
    // FR-127's variants, and:
    Reduced { reductions: Vec<AppliedReduction> },
}

pub enum AppliedReduction {
    Symmetry { groups: Vec<SymmetryClass>, orbit: Vec<String>, stabiliser: Vec<SymmetryClass> },
    PartialOrder { proviso: Proviso },
}

pub enum Proviso { BreadthFirstRevisit }

// InconclusiveCause gains:
//   ReductionNotPreserving { reduction: ReductionKind, form: PropertyForm },
//   SymmetryBroken { population: DeclarationKey, cause: SymmetryBrokenCause },
//   ConstraintReached { boundary_states: u64 },
//   ReductionHorizon { max_depth: u64 }
pub enum SymmetryBrokenCause {
    InitialStatesNotClosed { initial: u64, generator: Permutation },
}
pub enum ReductionKind { Symmetry, PartialOrder, StateConstraint }
```

## Behavior

- **Map rows.** FR-127's map SHALL gain these rows, with no `_` arm:

| Input | QSpec FR-360 label | QSpec FR-243 basis | QSpec FR-385 proof basis or cause | `TerminalValue` | O-16 category |
| --- | --- | --- | --- | --- | --- |
| `Holds` with symmetry or partial-order reduction applied | `proved` | `closed-scope` | proof basis `reduced`, label `uncertified` | `Proved{basis: Reduced{reductions}, certification: Uncertified}` | success |
| `Holds` with only a constraint, no boundary state reached | `proved` | `closed-scope` | proof basis `exhaustive` | `Proved{basis: Exhaustive}` | success |
| `ReductionNotPreserving{reduction, form}` | `inconclusive` | `unsettled` | `reduction-not-preserving` | `Inconclusive(ReductionNotPreserving{…})` | inconclusive |
| `SymmetryBroken{population, cause}` | `inconclusive` | `unsettled` | `symmetry-broken` | `Inconclusive(SymmetryBroken{…})` | inconclusive |
| `ConstraintReached{boundary_states}` | `inconclusive` | `unsettled` | `constraint-reached` | `Inconclusive(ConstraintReached{…})` | inconclusive |
| `ReductionHorizon{max_depth}` | `inconclusive` | `unsettled` | `reduction-horizon` | `Inconclusive(ReductionHorizon{…})` | inconclusive |

- **`Reduced` contents.** `reductions` SHALL list each applied reduction in
  the order symmetry, partial-order: for symmetry, the run's group, the
  orbit of keys the instance stands for, and the stabiliser it ran under;
  for partial-order reduction, the proviso. The record SHALL carry the
  subject's universes as the bounds.
- **Certification.** Every `Reduced` proof SHALL carry
  `certification: Uncertified`: EN-1's closure certificate does not cover a
  reduced graph, and no core checker for a reduced proof exists.
- **Category.** `TerminalValue::category` SHALL map every `Reduced` basis to
  success.
- **Budgets.** When a budget stops a run under any reduction before the run
  completes its method (`max_states`, `max_transitions`,
  `max_automaton_states`, the meter, the time budget, or cancellation), the
  map SHALL settle ADR-018 V-7 as `incomplete`, cause `limit-reached`,
  naming the budget, its value (the published default when the request sets
  none) and the request member that raises it, never `failed`.
- **Precedence.** When a run completes with no counterexample, reached a
  boundary state and reached `max_depth`, it SHALL settle
  `ConstraintReached`.
- **Horizon under partial-order reduction.** `max_depth` is the search
  horizon, a method parameter, and never a limit. A run with partial-order
  reduction that completes to `max_depth` with no counterexample SHALL
  settle `ReductionHorizon{max_depth}`, never `BoundReached` and never V-7.
  A symmetry-only run that completes to `max_depth` SHALL settle
  `BoundReached{depth}` (V-5).
- **Reduced-versus-unreduced agreement.** For every model-check case of
  QSL's model-check test corpus (the worked examples of ADR-018, ADR-019,
  ADR-021 and ADR-027, and the QSpec conformance vectors QSL runs) and every
  selection of symmetry, partial-order reduction, or both, whose PT-2 rows
  admit the case, the reduced run and the unreduced run SHALL settle the
  same ADR-018 verdict, and every counterexample from either run SHALL
  replay (FR-128). A case with a state constraint, and a pair in which
  either run ends at its horizon or at a limit, is not compared (RV-10).
- **Refutations.** A `Violated` outcome from a reduced run SHALL settle
  `refuted` through replay of its concrete counterexample (FR-161, FR-128),
  with basis `decisive-counterexample`, and its record SHALL name no
  reduction.
- **Undefined evaluation.** A `Violated` outcome whose counterexample has
  `kind: UndefinedEvaluation` SHALL settle as ADR-018 UE-1 to UE-6 state,
  through this same rule, with no reduction named (ADR-021 RV-7).
- **Instances.** The map SHALL record the verdict of an instance for every
  key of its orbit, each record naming its own `over` binding and the
  representative's run.
- **Identity.** Each record's obligation identity SHALL be the one FR-159
  writes, so a verdict on a reduced request never settles the unreduced
  request for the same claim, or the reverse.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-160-AC-1 | Each new map row settles as the table states: ADR-021 §7.1's `fair weak each` instance settles `proved`, `closed-scope`, `Proved{Reduced{[Symmetry{groups: [[a, b, c]], orbit: [a, b, c], stabiliser: [[b, c]]}]}, Uncertified}`, success, with three records, one per key of the orbit; §7.2's instance settles `Proved{Reduced{[PartialOrder{BreadthFirstRevisit}]}, Uncertified}`; FR-159-AC-1's TP-2 request settles `inconclusive`, `unsettled`, `ReductionNotPreserving{PartialOrder, TP-2}`; FR-151-AC-3's broken initial state settles `SymmetryBroken{config_history, InitialStatesNotClosed{0, (a b)}}`; FR-158-AC-2's runs settle `ConstraintReached{12}`. | Test (TC-584) |
| FR-160-AC-2 | Over ADR-021 §7.3's subject, `always holds(x.v <= 3)` with constraint `Wide` (`self.v <= 3`) reaches no boundary state and settles `Proved{Exhaustive}`, with `Wide` named in the method. `always holds(x.v <= 2)` with constraint `Low` and `max_depth` 2 completes depth 2, reaches boundary states at depth 2 and finds no violation; it settles `ConstraintReached`, not `BoundReached`. | Test (TC-584) |
| FR-160-AC-3 | ADR-021 §7.2's TP-4 instance with partial-order reduction and `max_depth` 2 settles `inconclusive`, `unsettled`, `ReductionHorizon{max_depth: 2}`, wire cause `reduction-horizon`, and never `failed` or `Incomplete`; with symmetry only over §7.1 and `max_depth` 1 it settles `BoundReached{depth: 1}`. | Test (TC-584) |
| FR-160-AC-4 | ADR-021 §7.1's `fair weak whole` refutation from the symmetry run settles `refuted`, `decisive-counterexample`, after replay, and its record names no reduction, and its counterexample, the three `upd(b)` steps from `(0,0,0)`, equals the unreduced run's counterexample byte for byte. | Test (TC-584) |
| FR-160-AC-5 | Over ADR-021 §7.1's subject with the symmetry declaration, `always holds(2 / (2 - c.versionNumber) >= 1)` for `c = a` settles `refuted`, `decisive-counterexample`, cause `UndefinedEvaluation` with `cause` `division-by-zero` at a state where `a.versionNumber = 2`, after its concretised counterexample replays; its record names no reduction, and its prefix has the length of the unreduced run's, two steps. | Test (TC-588) |
| FR-160-AC-6 | The differential test runs every model-check case of the corpus unreduced and under each selection of symmetry, partial-order reduction, or both, that its PT-2 rows admit, and every pair settles the same verdict with every counterexample replaying. It includes ADR-021 §7.1 to §7.6: §7.1's `fair weak each` instance `Holds` and `fair weak whole` instance `Violated` under symmetry and unreduced; §7.2's instances under partial-order reduction and unreduced; §7.4 `Holds` under its default scheduler constraints and `Violated` under `scheduling adversarial`; §7.5's `Gate` `Violated` under `fair weak go`; and each §7.6 vector `Violated`, each under partial-order reduction and unreduced. | Test (TC-886) |
| FR-160-AC-7 | ADR-021 §7.2's TP-4 instance with partial-order reduction and `max_states` 3 stops before completing its method and settles `incomplete`, cause `limit-reached`, naming `max_states`, its value 3 and the request member that raises it, never `failed`. §7.2's completed partial-order proof settles `Proved{Reduced{[PartialOrder{BreadthFirstRevisit}]}, certification: Uncertified}`. | Test (TC-584) |

## Dependencies

- ADR-021 RV-1 to RV-5, RV-7, RV-8, RV-10, SYM-6; ADR-018 V-1 to V-8; ADR-013 O-16
  and O-24 (amended as ADR-021's "Amendments" lists).
- [FR-069](FR-069-implement-typed-proof-result-envelope.md) (terminal record
  and category map), [FR-126](FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md),
  [FR-127](FR-127-settle-a-model-check-verdict-as-a-terminal-record.md) (the
  map this extends), [FR-159](FR-159-pre-check-selected-reductions-against-the-preservation-table.md).
- QSpec FR-385 owns proof basis `reduced` in the FR-331 terminal record and
  the new causes onto FR-360 and FR-243 (ADR-021 QS-6).

## References

- QSpec half: QSpec FR-385 (Linear STD-134; ADR-021 §9 QS-6). Owning ticket: Linear
  QSL-368.
