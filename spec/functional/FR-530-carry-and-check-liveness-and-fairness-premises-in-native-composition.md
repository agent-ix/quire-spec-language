---
id: FR-530
title: "Carry and check liveness dependencies and fairness premises in native composition"
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
  - target: ix://agent-ix/quire-spec-language/FR-529
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-489
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-491
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-492
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-348
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-176
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-380
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-177
    type: depends_on
---
# FR-530: Carry and check liveness dependencies and fairness premises in native composition

## Description

QSL SHALL compile, emit and read back QSpec FR-491's `composition-decl` over
checked component packages with QSpec FR-489's four premise lists kept
separate, and SHALL settle an `assume-guarantee` composition item from QSpec
FR-492's settlement request with FR-489's premise discharge record, in the
`qsl-replay` `planner` module FR-529 places; every other rule stays QSpec
FR-348's routed `composition` item.

## Inputs

- A composing unit that imports the component checked packages and declares
  a `composition-decl` (QSpec FR-491).
- For settlement: the composing checked package, its component packages, and
  one `quire.planner-settlement/v1` request (QSpec FR-492) carrying the
  slots, the pool of supplied component guarantee results and any discharge
  records.

## Outputs

- At S2 to S4: the checked declaration, its `correspondence`/
  `component_composition` node and its derived item and slots; or QSpec
  FR-491's located refusals.
- At settlement: QSpec FR-492's planner result with FR-489's premise
  discharge record, or its `invalid_runtime_input` refusal.

## Behavior

### Authoring and compile

The declaration is a `Relation` family declaration (ADR-012 §3), built at S2,
checked at S3 by QSpec FR-491 "Static checks", and emitted and read back at
S4; its targets in component packages resolve through the S4 dependency
closure, as a refinement's abstract package does (ADR-020 RM-1). S3 builds the
dependency graph and finds its cycles (strongly connected components) per
QSpec FR-491 "Dependency graph and cycles". Under `assume-guarantee` it
derives one planner-local item and its slots; under any other rule it
derives one FR-331 `composition` requirement record whose `premises` member
`qsl-route` writes from the slots, and `route` negotiates it as QSpec
FR-348 states, with `unsupported` when no composition-capable backend is
registered (FR-348-AC-4).

### Owning layer and input type

Settlement is FR-529's `qsl_replay::planner` module, for the reasons and
under the layer allow-list FR-529 states. `read_settlement_request` reads a
composition request into the same `SettlementRequest`, whose pool holds one
`PremiseResult` per supplied component guarantee and whose discharge records
pair a slot index with a pool index; QSpec FR-492 "Admission" applies,
including step 7 for a discharge record.

### What is reused from TL-9 and what QSL adds

QSL holds no native implementation of TL-9's assume-guarantee algorithm,
fixed point or protocol refinement, and adds none:

| Shape | Reused, as an input or a route | QSL addition |
| --- | --- | --- |
| Composition negotiation and the non-`assume-guarantee` rules | QSpec FR-348's `composition` item, routed unchanged; its result is read | the `premises` member the request writer fills |
| Acyclic discharge order | QSpec FR-176 "Discharge fixed point": acyclic nodes discharge in topological order | `planner` discharges slots in that order |
| Fixed point over a cycle | QSpec FR-176's result over the cycle, run by its provider, carrying every iteration (FR-176-AC-4) | the cycle detection, the profile check and the `fixed-point` slot that holds that result |
| Protocol refinement and refinement transfer | QSpec FR-177 and FR-380 results, read as premise results | none |
| Fairness implication | QSpec FR-380 "Fairness implication" | its application to each fairness premise |
| Premise transport and discharge record | none | the declaration, the four lists, the slots, admission, the discharge matching and the record |

### Discharge

`planner` discharges each slot by QSpec FR-489 "Implication": a supplied
guarantee discharges a `rely` or `liveness-dependency` slot only when its
request item's subject is the slot's peer subject, its claim node is the
peer guarantee clause, FR-363 gives it the slot's form, its result is
`proved`, and each of its fairness constraints is implied by a `supplies`
constraint. No other entailment is decided: a different clause, a weaker
claim, a guarantee of another kind, or a same-named guarantee of another
package leaves the slot undischarged. A component-guarantee slot is
discharged by a `proved` result of that guarantee's item checked under
exactly its FR-176 assumption set (the peer guarantees its `rely` and
`depends` members name), which is part of its slot key (QSpec FR-491); a
result checked under other assumptions or none is refused at admission, and
a failed component check leaves the slot undischarged. A discharge record that names an existing pool
entry over the right subject is used only after the same check; an
`inconclusive` referenced result discharges nothing.

### Cycles

An accepted cycle's premises are discharged only by its `fixed-point` slot's
`proved` FR-176 result; no guarantee of the cycle is available as an
assumption before it. A cycle with no profile declaring FR-176's lattice
refuses the composition item `invalid_model_binding`/
`malformed-declaration` naming the cycle (QSpec FR-491), and the package's
other items settle unchanged.

### Settlement and outcomes

The item settles by QSpec FR-492 "Decision order", writing QSpec FR-489's
premise discharge record: per slot, the discharging pool entry or supplied
constraint, the subject, the relation (`identity`, `fairness-implication`,
`assumption` or `fixed-point`), its certification and extent. A premise
discharged by a `trusted` plugin result settles `proved` with certification
`trusted`; a `rely` or liveness-dependency premise whose only discharger is
an environment record is `inconclusive`, `premise-unproved` with `result`
`assumption`, while an `environment` premise itself settles `supplied` and
is listed as an assumption. Unmet premise, trusted
assumption, unsupported rule, failed check and exhausted budget keep their
distinct outcomes, trust basis and extent; the request's unrelated supported
claims settle unchanged. Backend execution stays with the TL-9 providers,
and Quoin consumes the result without validating the theorem anew.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-530-AC-1 | Compiling, emitting and reading back a `composition-decl` keeps the component subjects, the rule and the `environment`, `relies`, `liveness_dependencies` and `fairness` lists separate, with each slot's source span its member; a Boolean success flag in place of the slots is refused `invalid_runtime_input`/`wrong-value-kind`. | Test (TC-1330) |
| FR-530-AC-2 | The discharge record names, for each slot, its discharger; a supplied guarantee over another subject, a weaker clause, a guarantee of another kind and a same-named guarantee of another package each leave the slot undischarged and the item `inconclusive`, `premise-unproved`, naming it. | Test (TC-1330) |
| FR-530-AC-3 | A `Producer`/`Consumer` fixture whose relies, dependencies and fairness are each discharged settles `proved`, `decisive-witness`, under `assume-guarantee` with the discharge record. | Test (TC-1330) |
| FR-530-AC-4 | A mutual liveness dependency with no profile declaring FR-176's lattice refuses the composition item `invalid_model_binding`/`malformed-declaration` naming both guarantees, and no guarantee of the pair is used as an assumption. | Test (TC-1330) |
| FR-530-AC-5 | The same pair under a declaring profile settles `proved` only when its fixed-point slot holds a `proved` FR-176 result, which the discharge record carries; with that slot `inconclusive` the item is `inconclusive`, `premise-unproved`, naming it. | Test (TC-1330) |
| FR-530-AC-6 | A `trusted` plugin discharger, an assumption-only premise, an unsupported rule (`interface` with no composition-capable backend), a failed component check and an exhausted `max_work` settle `proved`/`trusted`, `inconclusive`/`premise-unproved`, `unsupported`, `inconclusive`/`premise-unproved` and `incomplete`, each with its own trust basis and extent. | Test (TC-1330) |
| FR-530-AC-7 | A discharge record naming no pool entry, or a pool entry over another subject, is refused `invalid_runtime_input`/`invalid-value` at the record; one naming an `inconclusive` result discharges nothing. | Test (TC-1330) |
| FR-530-AC-8 | Admission refuses a forged or reordered slot and a premise result whose obligation identity is not its slot's; an unrelated supported item of the same package settles unchanged in every case. | Test (TC-1330) |

## Dependencies

- QSpec FR-489, FR-491 and FR-492 (with their TC-983, TC-985 and TC-986);
  QSpec FR-348, FR-176, FR-177 and FR-380, reused as the table above states.
- [FR-529](FR-529-settle-a-temporal-guarantee-of-implementation-rust-natively.md):
  the `planner` module, its input type and the closed causes.
- This requirement adds no parallel evidence store and no theorem import.

## References

- Linear QSL-1542; TL-9.
