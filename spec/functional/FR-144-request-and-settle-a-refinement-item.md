---
id: FR-144
title: "Request and settle a refinement item"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-018
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-014
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-018
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-020
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-075
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-123
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-124
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-127
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-142
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-143
    type: depends_on
---
# FR-144: Request and settle a refinement item

## Description

QSL SHALL write one requirement record for each refinement declaration and
settle it as exactly one FR-331 terminal record (ADR-020 §5 "Claim kinds",
"Strength" and "Negotiation", RE-4, RS-9). The record's kind is
`temporal-satisfaction`, its property form is TP-5 `Refinement` with a
safety half and, when `F_A` is non-empty, a liveness half. `model_check`
owns the map from a `RefinementOutcome` (FR-142, FR-143) and the replay of
its counterexample (FR-145) to `TerminalValue`, extending FR-127's map.

## Use case

A verification operator reads the result of a refinement request. A
`proved` refinement says it was proved by exhaustive exploration of the
concrete subject. A refuted one has a replayed counterexample. A
refinement whose liveness half QSL cannot decide says so and still reports
what the safety half found. The concrete model's own deadlocks are reported
beside it as a separate item.

## Inputs

- A `CheckedRefinement` in the S4 package (FR-135).
- The request's concrete subject and the abstract subject's initial
  snapshots.
- A `RefinementOutcome` (FR-142, FR-143), and for a `Violated` half the
  FR-072 result of replaying its counterexample (FR-145) or the
  `ReplayRefusal` that stopped it.

## Outputs

- A requirement record (`temporal-satisfaction`, `Unbounded{domains}`)
  with `PropertyForm::Refinement{liveness: bool}` (TP-5), where `liveness`
  is `true` exactly when `F_A` is non-empty. FR-123's `PropertyForm` gains
  this variant.
- `InconclusiveCause::MappingUndetermined` beside FR-127's causes, written
  on the wire as cause `mapping-undetermined` (QSpec FR-379).
- One FR-331 terminal record per refinement item, carrying the item's
  `TerminalValue`, each half's `TerminalValue`, the method
  `explicit-state`, its QSpec FR-360 label, QSpec FR-243
  basis and O-16 category by FR-127's table.

## Behavior

### Requirement record and request

- S3 SHALL write one requirement record for each refinement declaration,
  keyed by its occurrence key, with kind `temporal-satisfaction` and the
  TP-5 form. No other capability kind SHALL be written for it.
- Negotiation SHALL route the record by its kind and form to the
  explicit-state engine's provider (FR-075), whose manifest advertises
  `temporal-satisfaction`.
- The item's obligation identity SHALL bind the refinement node, the
  concrete subject and the abstract subject's initial snapshots (ADR-013
  O-09), so its verdict holds for exactly that concrete subject against
  that abstract subject with the derived universes.
- For FR-124's derivation the request writer SHALL count a refinement item
  as a temporal item over the concrete subject, so the request carries the
  concrete subject's deadlock-freedom item unless the concrete model
  declares `terminal any`. A refinement item SHALL NOT add a
  deadlock-freedom item for the abstract subject (ADR-020 RS-9).

### Settlement

- The item's value SHALL be:
  - the safety half's value when the safety half is not `Holds`;
  - `Proved{basis: Exhaustive}` when the safety half holds and `liveness`
    is `None`;
  - the liveness half's value when it is `Checked`;
  - `Unsupported(unsupported-requested-capability)` when it is
    `Unsupported`, with the record carrying the safety half's value
    `Proved{basis: Exhaustive}` beside it (ADR-020 AX-5).
- Each half's value SHALL come from FR-127's map, with
  `Undecided(MappingUndetermined)` settling V-6 `inconclusive`,
  `Inconclusive(MappingUndetermined)`, and an `UndefinedEvaluation`
  counterexample settling V-4 `refuted` with that cause once it replays
  (ADR-020 RE-5).
- A `Violated` half SHALL settle `refuted` only through its replay (FR-145)
  settling `reproduced-with-evaluated-witness`; a replay that settles
  `inconclusive` SHALL settle V-6 with `ReplayParity` (wire cause
  `replay-parity`, QSpec FR-379), a refusal V-6 with
  `ReplayRefused`, and a fault `failed`.
- The record SHALL read neither model's `terminal` member; its value SHALL
  be the same under every combination of them.
- **Certificate.** A safety half that the product proves SHALL settle
  `proved` only after FR-149's checker accepts its certificate; a rejection
  SHALL settle V-6 `CertificateRejected{rule, state}` (ADR-018 PC-2), and a checker stopped by a limit
  V-7 naming the limit (ADR-020 CT-3).
- **Certification label.** A `proved` record SHALL carry ADR-018 PC-1's
  `Certified` label when every half was certified. A `proved` with a
  liveness half, which has no core certificate checker, SHALL settle
  `proved` labelled `Uncertified` (ADR-020 CT-4, RU-7).

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-144-AC-1 | `CasRefinesCounter` writes one requirement record (`temporal-satisfaction`, `PropertyForm::Refinement{liveness: true}`); with its `ensure` row removed, `liveness: false`. The record routes to the explicit-state provider. The request carries the concrete subject's deadlock-freedom item and none for the abstract subject; with `terminal any` on `Impl::Counter` it carries none. | Test (TC-549) |
| FR-144-AC-2 | `CasRefinesCounter` settles `proved`, `closed-scope`, `Proved{basis: Exhaustive}`, method `explicit-state`, both halves `Proved{basis: Exhaustive}`. The divergence variant settles `refuted`, `decisive-counterexample`, after its lasso replays, with safety half `Proved{basis: Exhaustive}` and liveness half `Refuted`. | Test (TC-549) |
| FR-144-AC-3 | FR-142-AC-4's `writes` variant settles `inconclusive`, `unsettled`, `Inconclusive(MappingUndetermined)`, category inconclusive; FR-142-AC-6's `only` variant settles `refuted`, `decisive-counterexample`, cause `UndefinedEvaluation`, after FR-145 replay reproduces it. FR-143-AC-4's hidden-field refinement with its `ensure` row settles `unsupported`, `unavailable`, `Unsupported(unsupported-requested-capability)`, with the safety half `Proved{basis: Exhaustive}` on the record. | Test (TC-549) |
| FR-144-AC-4 | The lost-update refinement whose counterexample is replayed with its recorded failure changed to `AbstractStepRejected{position: 3, …}` settles `inconclusive`, `Inconclusive(ReplayParity)`, never `refuted`; replayed with an `initial` index of 1 over its one-snapshot subject, it settles `inconclusive`, `Inconclusive(ReplayRefused)`. | Test (TC-549) |
| FR-144-AC-5 | Two `CasRefinesCounter` requests that differ only in the abstract subject's initial snapshots (`value` 0 and `value` 1) have different obligation identities; the second settles `refuted` with `InitialNotAbstract`. | Test (TC-549) |
| FR-144-AC-6 | FR-140-AC-5's incomplete `items` row settles `inconclusive`, `unsettled`, `Inconclusive(MappingUndetermined)` (wire cause `mapping-undetermined`), never `refuted`; FR-141-AC-7's undefined argument settles `refuted`, `decisive-counterexample`, cause `UndefinedEvaluation`, after FR-145 replay reproduces it. | Test (TC-555) |
| FR-144-AC-7 | `CasRefinesCounter` with its `ensure` row removed settles `proved`, `closed-scope`, `Certified`, after FR-149 accepts the certificate; with its `ensure` row it settles `proved`, `Uncertified`; with the certificate's `Taken` for the `commitA` edge from `(0, 0, t, 0, f)` changed to `Stutter`, it settles `inconclusive`, `CertificateRejected`. | Test (TC-556) |

## Dependencies

- ADR-020 §2 RS-9, §4 AX-5, §5 RE-1, RE-4, "Claim kinds", "Strength" and
  "Negotiation"; ADR-018 §1 V-1 to V-8 and §10 DL-3; ADR-013 O-09 and O-16;
  ADR-014 A-3.
- [FR-075](FR-075-compute-candidates-from-registered-backends.md),
  [FR-123](FR-123-check-fairness-and-interval-operators-of-infinite-trace-clauses.md),
  [FR-124](FR-124-declare-intended-terminal-states-and-derive-deadlock-freedom.md),
  [FR-127](FR-127-settle-a-model-check-verdict-as-a-terminal-record.md),
  [FR-142](FR-142-check-a-refinement-s-safety-half-on-the-explicit-state-product.md),
  [FR-143](FR-143-check-a-refinement-s-liveness-half-under-abstract-fairness.md).

## References

- The QSpec half (claim kinds and the `mapping-undetermined` cause): QSpec FR-379 (Linear STD-133).
