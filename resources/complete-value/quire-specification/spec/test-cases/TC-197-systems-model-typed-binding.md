---
id: TC-197
title: "Systems-model typed binding"
type: TC
relationships:
  - target: ix://agent-ix/quire-specification/FR-152
    type: verifies
---

# TC-197: Systems-model typed binding

## Description

Resolve every model element kind and attempt kind, authority and closure substitutions.

## Test Procedure

Use the exact version-locked subject and inputs named by FR-152. Execute its positive case, each declared boundary, and each explicit refusal/non-conclusive mutation through the real local Rust boundary; compare the complete typed result and artifact accounting with the criterion.

Select `quire.model.complete/v1` at revision `1-draft.1` by its exact
DefinitionRefs in
[`complete-model-lock.json`](../../proposals/quire-v1/definitions/complete-model-lock.json).
Producer keys, header closure, multiplicity notation and default limits follow
TC-195 and TC-196; the source is TC-195 S1 importing `bundle.y`, with profile
alias `V` and model alias `M`. A binder request is written `resolve(kind,
name)`.

Fixture **Y** (bundle `bundle.y`):

- scalar type export `model.Count`, bound as `Int[0,9]`;
- object types `model.Sys`, `model.Pump`, `model.Tank`; Interface types
  `model.Flow` and `model.Flow2`, each with `interfaceFeatures: [{featureIdentity:
  model.Flow.rate, featureKind: field}]`, generalization `model.gen.Flow2-Flow`
  and field `model.Flow.rate` typed `model.Count` `{1,1}`;
- Parts: components `model.Sys.pump` (`owningTypeIdentity: model.Sys`,
  `typeIdentity: model.Pump`, `{1,1}`) and `model.Sys.tank` (`model.Tank`,
  `{1,1}`);
- Ports: endpoints `model.Sys.pump.out` (`owningComponentIdentity:
  model.Sys.pump`, `typeIdentity: model.Flow`, `direction: out`, `{1,1}`) and
  `model.Sys.tank.in` (`model.Sys.tank`, `model.Flow`, `in`, `{1,1}`);
- Connection: relationship `model.Sys.pipe`, source end `model.Sys.pump.out`,
  target end `model.Sys.tank.in`, both `{1,1}`, `semantics.category:
  connection`, `semantics.direction: source-to-target`;
- operation `model.Pump.run` with no parameters, no result and an empty effect;
- Allocation: relationship `model.Pump.alloc`, `semantics.category: allocation`,
  source end `typeIdentity: model.Pump.run`, target end `typeIdentity:
  model.Sys.pump`;
- navigation: relationship `model.rel.parts` with source end role `owner`
  (`model.Sys`, `{1,1}`), target end role `parts` (`model.Pump`, `{0,3}`),
  category `composition`, direction `source-to-target`; and relationship
  `model.rel.home` with source end role `pump` (`model.Pump`, `{1,1}`),
  target end role `home` (`model.Tank`, `{1,1}`), direction `source-to-target`.

Population **Q**: `closedWorld: true`, object `s1` of type `model.Sys` whose
`parts` instances target `p2` and `p1`, objects `p1`, `p2` of type
`model.Pump` whose `home` targets `t1` of type `model.Tank`.

Ownership: the Parts, both Ports and the Connection `model.Sys.pipe` (both ends
owned by `model.Sys`) are effective members of `Sys`; the Allocation's source
end-owner type is `model.Pump` (owner of `model.Pump.run`) and its target
end-owner type is `model.Sys` (owner of `model.Sys.pump`), so it is qualified
into both and resolves as the member owned by `Pump`.

| Vector | Input | Expected result/disposition |
| --- | --- | --- |
| Y01 | `resolve(Part, M::Sys::pump)`, `resolve(Port, M::Sys::pump::out)`, `resolve(Interface, M::Flow)`, `resolve(Connection, M::Sys::pipe)`, `resolve(Allocation, M::Pump::alloc)` | each returns its kind with the exact producer key and effective declaration identity: Part `(Sys, model.Sys.pump)`, Port `(Sys, model.Sys.pump.out)`, Interface the effective type `Flow`, Connection `(Sys, model.Sys.pipe)` and Allocation `(Pump, model.Pump.alloc)`; `M::Sys::alloc` does not name the Allocation, which is exposed only as the member owned by its source end-owner type; the Connection carries source `model.Sys.pump.out` and target `model.Sys.tank.in`; the Allocation carries source `model.Pump.run` and target Part `model.Sys.pump` |
| Y02 | `resolve(Port, M::Sys::pump)`; `resolve(Allocation, M::Sys::pipe)`; Y with `model.Pump.alloc`'s target end `typeIdentity` replaced by `model.Sys.pump.out` | `refused { code: invalid_model_binding, cause: wrong-export }` with required `Port`, actual `Part`; the same with required `Allocation`, actual `Connection`; the same for the allocation target with required `Part`, actual `Port` |
| Y03 | Y with `model.Sys.pipe` changed to: (a) source `model.Sys.tank.in`, target `model.Sys.pump.out`; (b) direction `target-to-source`; (c) direction `bidirectional`; (d) (c) with both ports `inout`; (e) direction `undirected` | (a) `refused { code: invalid_model_binding, cause: port-direction }` with direction `source-to-target`, source `in`, target `out`; (b) the same refusal: the flow source is the target end `in`; (c) the same refusal: both ports must be `inout`; (d) admitted, both interface types being `model.Flow`; (e) the same refusal, `undirected` never being a connection direction |
| Y04 | Y with `model.Sys.tank.in` typed `model.Flow2`; then instead `model.Sys.pump.out` typed `model.Flow2` | `refused { code: ill_typed, cause: type-mismatch }` with flow-source interface `model.Flow` not conforming to `model.Flow2`, the interface condition charging `f(Flow) = 1` work unit, so the three `systems.connection-condition` charges consume three work units; admitted, `model.Flow2` conforming to `model.Flow`, the interface condition charging `f(Flow2) = 2` (its qualify and inherit facts), so the three charges consume four work units |
| Y05 | Y with `model.Sys.pipe`'s target end multiplicity `{0,2}` | `refused { code: ill_typed, cause: multiplicity-narrowing }` with expected port multiplicity `{1,1}` and actual end `{0,2}`; no connection is exposed |
| Y06 | Y with `model.Sys.pump`'s `typeIdentity` member removed | after all `systems.kind` and `systems.allocation` charges, four `invalid_model_binding` refusals in rule order: (1) kind mapping, `unsupplied-producer-record` with `{capability: part-signature, required: 1.3.0, supplied: 1.3.0, item: model.Sys.pump}`; (2) kind mapping, `wrong-export` for endpoint `model.Sys.pump.out` with required `Part` and actual `none` (its component has no kind); (3) kind mapping, `wrong-export` for the source end of `model.Sys.pipe` with required `Port` and actual `none`, so `model.Sys.pipe` has no kind and no `systems.connection-condition` charge; (4) allocation rule, `wrong-export` for the target `model.Sys.pump` of `model.Pump.alloc` with required `Part` and actual `none`; `resolve(Part, M::Sys::pump)` has no result |
| Y07 | Over Q, `self.parts` in `invariant I using V on M::Sys at current { ... }` with `self` = `s1`; then with every `ScalarLimitsV1` counter unlimited except `work_units: 5`, then `4`; then Q plus objects `p3` and `p4` of type `model.Pump` with `s1.parts` targeting `p1`, `p2`, `p3`, `p4`; then the same `s1.parts` targets with `p4` absent from the closed population | `Set<Reference<M::Pump>>[0,3]` equal to `[p1, p2]` in reference-key order, after `model.navigate` (`value_occurrences` 1), two `collection.visit`, `collection.bound` (`value_occurrences` 2) and `collection.result-retain` (`value_occurrences` 3, `result_units += 3`): five work units and three result units; the `5` run completes identically; `incomplete { limit_kind: work_units, limit: 4, consumed: 4, next_charge: 1, charge_point: collection.result-retain }`; `refused { code: cardinality_out_of_bound, cause: above-maximum }` with bound `[0,3]` and count 4 after `model.navigate`, four `collection.visit` and `collection.bound`: exactly six work units and no result unit, with no `collection.result-retain` and no value; `refused { code: dangling_reference, cause: absent-target-in-complete-population }` for `p4` after `model.navigate` and three `collection.visit` (`p1`, `p2`, `p3`): exactly four work units, decided before `collection.bound`, so no cardinality refusal |
| Y08 | `self.home` with `self` = `p1`; `self.pump` with `self` = `t1` in an invariant on `M::Tank`; Y with `model.rel.home` direction `undirected`, then `self.pump` again; Y with an added field member `model.Pump.home` typed `model.Tank`, then `self.home` | `Reference<M::Tank>` equal to `t1` after one `model.navigate`: one work unit and no result unit; `refused { code: ill_typed, cause: operator-ineligible }`, the traversal running target to source against `source-to-target`; the navigation checks, `undirected` admitting both traversals, and evaluating it for `t1` is `refused { code: cardinality_out_of_bound, cause: above-maximum }` with bound `[1,1]` and count 2 (`p1` and `p2`) after one `model.navigate`; `refused { code: ambiguous_declaration, cause: ambiguous-name }` listing the field and the relationship end |
| Y09 | Y with `model.rel.parts` target end `{0,unbounded}`; then `{0,3}` with `ordered: true`; then Q with `p2`'s `home` targeting `t9` absent from the closed population; then Y with `model.rel.home`'s target end `typeIdentity` naming a type export of bundle `bundle.other` | `refused { code: unsupported_construct, cause: expression-form }` at `self.parts`; the same refusal; `refused { code: dangling_reference, cause: absent-target-in-complete-population }` for `t9`; `refused { code: foreign_reference, cause: foreign-model-selection }` with required `bundle.y` and supplied `bundle.other` selections at checking |

## Expected Results

Every positive and boundary result matches FR-152; each invalid, unsupported or exhausted mutation returns its exact typed disposition with source/provenance, emits no approximation, and preserves independent sibling results.
