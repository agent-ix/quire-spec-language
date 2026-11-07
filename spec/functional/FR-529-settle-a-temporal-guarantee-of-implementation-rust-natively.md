---
id: FR-529
title: "Settle a temporal guarantee of implementation Rust from checked correspondence and progress premises"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-031
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-012
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-017
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-020
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-290
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-304
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-306
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-307
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-488
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-490
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-492
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-353
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-350
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-380
    type: depends_on
---
# FR-529: Settle a temporal guarantee of implementation Rust from checked correspondence and progress premises

## Description

QSL SHALL compile, emit and read back QSpec FR-490's `guarantee-decl`, derive
its premise items and slots, read QSpec FR-492's settlement request for its
implementation-guarantee item, and settle that item only through QSpec
FR-488's settlement, in the layers ADR-011 assigns below.

## Inputs

- A unit with a `guarantee-decl` (QSpec FR-490) over one of its
  infinite-trace temporal clauses and one of its `abstraction-decl`s
  (FR-304).
- For settlement: the checked package that carries it and one
  `quire.planner-settlement/v1` request (QSpec FR-492) carrying every
  derived slot, the complete FR-331 request item and result entry behind
  each filled slot, and each environment record.

## Outputs

- At S2 to S4: the checked declaration, its `correspondence`/
  `implementation_guarantee` node, and its derived requirement records; or
  QSpec FR-490's located refusals.
- At settlement: QSpec FR-492's planner result (with FR-488's conclusion
  record when `proved`), or its `invalid_runtime_input` refusal.

## Behavior

### Authoring and compile

The declaration is a `Relation` family declaration beside the abstraction
relation (ADR-012 §3, ADR-017 AR-1). S2 (`qsl-forms`) builds its form; S3
(`qsl-semantics`, the `Relation` family checker) applies QSpec FR-490 "Static
checks" in order and derives QSpec FR-490 "Derived items"; S4 (`qsl-package`)
emits and reads back the `correspondence`/`implementation_guarantee` node.
Each derived `operation-contract` item is a requirement record of the
checked package carrying QSpec FR-331's `correspondence` member, and the
model-claim item carries `stutter_closure` for a liveness clause; `qsl-route`
writes both members into the requested item (ADR-013 O-20), so `route` and
E7 negotiate them as every other item (ADR-012 §7.2). An unbound model
element refuses only the guarantee item, through FR-307's per-item export.

**The abstraction relation stays inert.** FR-306-AC-3 and FR-306-AC-4 are
unchanged: a relation records no requirement record and changes no `route`
result or clause disposition. Only a `guarantee-decl` that names the relation
derives items; a unit with the relation and no `guarantee-decl` derives what
it derives without the relation (QSpec FR-490-AC-8), so
`tests/it/abstraction_relation_inert.rs` keeps passing unchanged.

### Owning layer of admission and settlement

Admission and settlement are a new module `planner` of layer 6 `qsl-replay`,
not of `qsl-route` (R) or `qsl-analyze` (A):

- they read QSpec FR-331 request items and result entries into
  `TerminalRecord`s, which `qsl-replay` owns (ADR-013 O-24), through its
  existing result readers (FR-290 for a plugin entry);
- they read the checked package (layers 3 and 4) and nothing above layer 6;
- `qsl-replay` already settles an item from other items' results
  (QSpec FR-468's hypothesis settlement in `model_check::step_safety`).

ADR-011 §6.1's layer-6 row allows every edge `planner` needs, so no
`[dependencies]` table changes and TC-390's tables hold. `qsl-route` gains
only the two request members above; the orchestrating driver (ADR-011 T-13)
collects the premise results and calls `planner`.

### Premise-result input type

`qsl_replay::planner::SettlementRequest` is the typed value of one FR-492
request, built only by `planner::read_settlement_request(&CheckedPackage,
&[u8], ReadLimits)`, which applies QSpec FR-492 "Admission" in order and
returns QSpec FR-272's `invalid_runtime_input` cause and pointer for the
first defect. It holds the declaration's node, the slots in derived order,
and per filled slot one `PremiseResult`:

| Field | Meaning |
| --- | --- |
| `identity` | the slot's FR-201 `ObligationIdentity` (`qsl_replay::identity`), which binds the package identity and so the relation revision |
| `item` | the premise request item as read: subject, claim node, FR-290 kind, fairness set, `stutter_closure`, `correspondence` and FR-331 extent classification |
| `record` | the item's `TerminalRecord`, read from the complete result entry |
| `certification` | the record's FR-354 `Certification` when `proved` |

An environment slot holds an `EnvironmentRecord {name, supplier,
supplies}` and no certification. A slot is constructed only from read bytes,
never from a bare label or a reference to another result.
`planner::settle(&CheckedPackage, &SettlementRequest, Budget)` returns the
`PlannerResult`.

### What is reused and what QSL adds

| Premise | Existing owner, reused | QSL addition |
| --- | --- | --- |
| Abstraction binding | FR-304, FR-306, FR-307 (QSL-36) | none |
| Per-step and correspondence contracts on bound Rust functions | the FR-350 Verus backend (IR-32): its FR-331 results are read, not re-derived | the derived `operation-contract` records with `correspondence` |
| Model claim over the stutter closure | QSpec FR-380 rule; ADR-020 CO-3's EN-1 stutter-edge construction | the `stutter_closure` request member |
| Fairness implication | QSpec FR-380 "Fairness implication" | the planner's fairness-correspondence settlement |
| No-divergence | none | the declaration's measure or bound, its derived record and its settlement |
| Environment premises | none | the typed environment record and slot |
| Settlement | QSpec FR-488's table | `planner`, through FR-492's decision order |

### Settlement

- The fairness-correspondence slot is settled by `planner` itself: `proved`,
  `certified`, when a scheduling environment premise over the constraint's
  operation (which has a frame binding) implies it under QSpec FR-380's kind
  and granularity rule; otherwise `Inconclusive(TransferNotPreserving {
  reason: Fairness { constraint } })`.
- The item settles by QSpec FR-492 "Decision order": the first slot that is
  neither `proved` nor `supplied` decides; an `unsupported` slot settles the
  item `unsupported` and an `incomplete` slot `incomplete`, each with that
  slot's cause; any other outcome settles `Inconclusive(PremiseUnproved {
  premise, result })`. A `tested` slot, a finite trace, never promotes the
  item to `proved`, and no guarantee item is `refuted`.
- Settlement of one item settles every other item of the package unchanged.

### Closed causes

`InconclusiveCause` (ADR-018 §1's closed set) gains the two QSpec FR-331
planner-only causes, with their FR-331 spellings:

- `PremiseUnproved { premise: u64, result: Option<ItemLabel> }`, spelled
  `premise-unproved{premise, result}`: `premise` the FR-492 slot index (for a
  transfer item, the premise item's `request_index`), `result` its label, or
  `None` for an unfilled slot, spelled `absent`;
- `TransferNotPreserving { reason }`, spelled
  `transfer-not-preserving{reason}`, `reason` `Fragment { operator }`,
  `Fairness { constraint }` or `StutterClosure`.

Only `planner` constructs them. FR-290's plugin reader refuses both in a
plugin's entry as QSL-only causes (`unknown-cause`), not as `Unsupported`.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-529-AC-1 | A valid supported chain yields a `proved` planner result, `decisive-witness`, whose conclusion record names the relation revision and implementation sources, the clause and fairness set, each slot outcome, the environment assumptions, the extent and each premise's certification, bound to the checked package's source map. | Test (TC-1329) |
| FR-529-AC-2 | A Rust step that violates the abstraction relation (a refuted transition-simulation result), and a liveness chain with only per-step contracts and no `progress` member, each leave the item `inconclusive`, `premise-unproved`, naming the transition-simulation slot and the first no-divergence slot respectively. | Test (TC-1329) |
| FR-529-AC-3 | An unmeasured stutter loop and an infinite internal-step behaviour (a not-proved no-divergence result) settle the item `inconclusive`, `premise-unproved`, naming the no-divergence slot; an `F_P` constraint stronger than every scheduling premise settles the fairness-correspondence slot `inconclusive`, `transfer-not-preserving{reason: fairness}`, and the item `premise-unproved` naming that slot. | Test (TC-1329) |
| FR-529-AC-4 | An unbound model element refuses the guarantee item `missing_declaration`/`missing-name`; an unregistered Verus backend settles it `unsupported` naming the capability; an exhausted bound settles it `incomplete` naming the limit; the three are distinct, and the package's other items settle unchanged. | Test (TC-1329) |
| FR-529-AC-5 | Compiling, emitting and reading back a `guarantee-decl` yields the same declaration, derived records and identities, each with the source span of the member that produced it; the declaration's static refusals are QSpec FR-490's, in its order. | Test (TC-1329) |
| FR-529-AC-6 | `read_settlement_request` refuses a forged, reordered or missing slot, a premise result whose obligation identity is not its slot's (another subject or relation revision included) and a listed environment premise with no record, each `invalid_runtime_input`/`invalid-value` at the member, and a Boolean success flag `invalid_runtime_input`/`wrong-value-kind`. | Test (TC-1329) |
| FR-529-AC-7 | A slot is filled only from a complete FR-331 request item and result entry; a slot holding only a reference to another result, or a bare label, is refused at admission. | Test (TC-1329) |
| FR-529-AC-8 | A finite implementation trace result (`tested`) over which the clause holds does not settle the item `proved`, and no guarantee item settles `refuted`. | Test (TC-1329) |
| FR-529-AC-9 | A unit with an abstraction relation and no `guarantee-decl` has the requirement records and `route` results it has without the relation (FR-306-AC-3 unchanged). | Test (TC-1329) |
| FR-529-AC-10 | `read_plugin_inconclusive_cause` refuses `premise-unproved` and `transfer-not-preserving` with their valid payloads as QSL-only causes (`unknown-cause` at `/cause/type`), and `planner` writes them with QSpec FR-331's spellings. | Test (TC-1329) |

## Dependencies

- QSpec FR-488, FR-490 and FR-492 (with their TC-982, TC-984 and TC-986):
  the semantics, the authored form and the request this requirement
  implements; QSpec FR-353, FR-350, FR-354 and FR-380.
- [FR-304](FR-304-check-an-authored-abstraction-relation.md),
  [FR-306](FR-306-carry-the-abstraction-relation-in-the-checked-package.md),
  [FR-307](FR-307-export-the-bindings-each-item-references.md): the bindings.
- [FR-290](FR-290-settle-plugin-results-as-typed-terminal-records.md): the
  plugin result reader whose cause lists this requirement amends.
- [ADR-011](../decisions/ADR-011-stage-dag-and-dependency-architecture.md)
  §6.1: the layer allow-list the placement above satisfies.
- Real Verus execution stays dependent on IR-32. This requirement adds no
  parallel evidence store and no new universal prover.

## References

- Linear QSL-1541; QSL-36.
