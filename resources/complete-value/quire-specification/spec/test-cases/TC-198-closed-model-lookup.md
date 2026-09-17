---
id: TC-198
title: "Closed model lookup"
type: TC
relationships:
  - target: ix://agent-ix/quire-specification/FR-153
    type: verifies
---

# TC-198: Closed model lookup

## Description

Query lookup/all-instances with complete, unknown, foreign and over-bound populations.

## Test Procedure

Use the exact version-locked subject and inputs named by FR-153. Execute its positive case, each declared boundary, and each explicit refusal/non-conclusive mutation through the real local Rust boundary; compare the complete typed result and artifact accounting with the criterion.

Select `quire.model.complete/v1` at revision `1-draft.1` by its exact
DefinitionRefs in
[`complete-model-lock.json`](../../proposals/quire-v1/definitions/complete-model-lock.json).
Producer keys, header closure and default limits follow TC-195. The model is
TC-195 F1 imported as `M` by TC-195 S1's shape with profile alias `V`: object universe `U` = `0873083c`, effective types
`A` = `f1cc59cd` and `B` = `b9953c43`. Because `b9953c43` precedes `f1cc59cd`,
every `B` member precedes every `A` member in reference-key order.

Population binding `p` has role `population`, anchor `current`, declared
maximum 3 and is bound to FCD FR-121 document **P1**: `closedWorld: true`,
`modelIdentity` naming F1's ModelSelection, members `a1` and `a2` of
`typeIdentity: model.A` and `b1` of `model.B`. Binding `q` is the same shape
over document **P2** with one member `c9` of `model.A`. `ra`, `rb` and `rc` are
`Reference<M::A>`, `Reference<M::B>` and `Reference<M::A>` parameters valued
`a1`, `b1` and `c9`.

Graph population binding `g` has role `population`, anchor `current`, declared
maximum 3 and is bound to document **P3**: `closedWorld: true`, F1's
ModelSelection, members `a1` of `model.A` with `x` targeting `a2`, `a2` of
`model.A` with `x` absent, and `b1` of `model.B` with `x` targeting `b1`.
`ga1`, `ga2` and `gb1` are `Reference<M::A>` parameters valued `a1`, `a2` and
`b1` of P3, and `gz` is a `Reference<M::A>` parameter valued `z9`, absent from
P3. Every `ScalarLimitsV1` counter not stated is unlimited.

| Vector | Input | Expected result/disposition |
| --- | --- | --- |
| L01 | `allInstances<M::A>(p)`; then with every `ScalarLimitsV1` counter unlimited except `work_units: 5`, then `4`; `allInstances<M::B>(p)` | `Set<Reference<M::A>>[0,3]` equal to `[b1, a1, a2]`, where `b1`'s triple is `(U, B, "b1")`, after three `population.visit`, `collection.bound` (`value_occurrences` 3) and `collection.result-retain` (`value_occurrences` 4, `result_units += 4`): five work units and four result units; the `5` run completes identically; `incomplete { limit_kind: work_units, limit: 4, consumed: 4, next_charge: 1, charge_point: collection.result-retain }`; `[b1]` of type `Set<Reference<M::B>>[0,3]` after three `population.visit` (every member walked, `a1` and `a2` not selected), the bound and a retain of two occurrences: five work units and two result units, with `b1`'s triple identical to its triple in the `M::A` result |
| L02 | P1 with `closedWorld: false`; then F1's header with `generalizationClosure: open` | binding admission returns the incomplete outcome `incomplete_population` with cause `incomplete-scope`, not a refusal, and no charge or collection; the incomplete outcome `incomplete_population` with cause `unclosed-subtypes` naming F1's ModelSelection and `M::A`, with no charge or collection |
| L03 | `lookup<M::A>(p, rb) absent undefined`; `lookup<M::A>(p, rc) absent undefined`; `lookup<M::A>(p, rb) absent empty`; `lookup<M::A>(p, rc) absent empty`; `lookup<M::A>(p, rc) absent refused`; `lookup<M::B>(p, ra) absent empty` | `b1` as `Reference<M::A>` after `lookup.key` and `lookup.result-retain`: two work units, one result unit; `undefined` with the catalogued reason `absent-key` after one work unit; present `b1` as `Option<Reference<M::A>>` after two work units and two result units; `none` after two work units and one result unit; `refused { code: invalid_runtime_input, cause: absent-key }` naming `p` and `c9` after one work unit; `refused { code: ill_typed, cause: type-mismatch }` before any charge, `A` not conforming to `B` |
| L04 | `lookup<M::A>(p, rx) absent empty` where `rx` is valued `a1` from a closed population of TC-195 F2 | `refused { code: foreign_reference, cause: foreign-universe }` with required `0873083c` and supplied `ed29d710`, after one work unit |
| L05 | P1 plus a second member record `a1` of `model.B`; P1 plus an exact duplicate of member `a1` with an equal member digest, then `allInstances<M::A>(p)`; P1 plus member `z1` of `typeIdentity: model.Z`, absent from F1 | binding admission `refused { code: invalid_runtime_input, cause: conflicting-identity }` listing both `a1` records, after four `binding.member` charges (`population_members` 4) and decided after the fourth; the duplicate collapses at the fourth `binding.member` charge, admission consumes four admission work units, and L01's evaluation result and charges recur; `refused { code: foreign_reference, cause: foreign-type }` for `model.Z` at binding admission after four `binding.member` charges |
| L06 | `p` with declared maximum 2, then `allInstances<M::A>(p)`; the same with only `work_units: 3`; a population binding with no declared maximum used as `allInstances<M::A>(p0)`; `allInstances<M::Count>(p)` for a scalar type | `refused { code: cardinality_out_of_bound, cause: above-maximum }` with bound `[0,2]` and count 3, after three visits and `collection.bound`: four work units and no result unit; `incomplete { limit_kind: work_units, limit: 3, consumed: 3, next_charge: 1, charge_point: collection.bound }`; `refused { code: ill_typed, cause: operator-ineligible }`; `refused { code: ill_typed, cause: type-mismatch }` |
| L07 | F1 plus operation member `model.A.remove` with no parameters, no result and effect `{fieldWrites: [], creates: [], deletes: [model.A]}`; an invocation deleting `a2`, whose pre population is P1 and post population is P1 without `a2`; in `post Q using V on M::A::remove { ... }`: `allInstances<M::A>(p)`, `pre(allInstances<M::A>(p))`, `lookup<M::A>(p, r2) absent empty` and `pre(lookup<M::A>(p, r2) absent empty)` for `r2` valued `a2` | `[b1, a1]`; `[b1, a1, a2]` with `a2`'s triple `(U, A, "a2")`; `none`; present `a2` with its pre type `A` |
| L08 | `(lookup<M::A>(p, rb) absent refused) = rb`; then over TC-195 F2 with `rb: Reference<M::B>` and `rk: Reference<M::C>`, `rb = rk` | `true` through the FR-149 `Reference<M::B>` to `Reference<M::A>` upcast, which charges nothing: two lookup work units and one result unit, then the five-unit reference equality: seven work units and two result units; `refused { code: ill_typed, cause: type-mismatch }` before any charge, neither type conforming to the other |
| L09 | Sort the `reference_key_order` entries of the vectors file as FR-144 keys, starting from any permutation | exactly the published order: `(0873083c, b9953c43, "o1")`, `(0873083c, f1cc59cd, "o")`, `(0873083c, f1cc59cd, "o1")`, `(0873083c, f1cc59cd, "o10")`, `(0873083c, f1cc59cd, "o2")`, `(ed29d710, f1cc59cd, "o1")`: universe first, then type, then object bytes with a proper prefix first; each identity component compared by its FR-204 key bytes (the UTF-8 domain string followed by its 32 digest bytes, one byte string) and `object` by its exact UTF-8 bytes |
| L10 | Over `g`, `reaches(ga2, ga2, M::A::x)`, `reaches(gb1, gb1, M::A::x)`, `reaches(ga1, ga2, M::A::x)` and `reaches(ga1, ga1, M::A::x)`; then the first with `work_units: 1`, the second with `work_units: 2`, and the fourth with `value_occurrences: 1` | `false` after `graph.expand` for `a2` and `graph.result-retain`: two work units and one result unit (the start is discovered but not reached, and `a2` has no `x` edge); `true` after `graph.expand` for `b1`, one `graph.edge` to `b1` and `graph.result-retain`: three work units; `true` after `graph.expand` for `a1`, one `graph.edge` to `a2` and `graph.result-retain`: three work units; `false` after `graph.expand` for `a1` (`value_occurrences` 1), `graph.edge` to `a2`, `graph.expand` for `a2` (`value_occurrences` 2) and `graph.result-retain`: four work units, because no edge returns to `a1` (FR-043-AC-2); `incomplete { limit_kind: work_units, limit: 1, consumed: 1, next_charge: 1, charge_point: graph.result-retain }`; `incomplete { limit_kind: work_units, limit: 2, consumed: 2, next_charge: 1, charge_point: graph.result-retain }`; `incomplete { limit_kind: value_occurrences, limit: 1, consumed: 1, next_charge: 2, charge_point: graph.expand }`; no incomplete run exposes a Boolean |
| L11 | Over `g`, `deref(ga1).x`, then with `work_units: 1`; `deref(gz)`, then with `work_units: 0` | present `a2` as `Option<Reference<M::A>>` after `model.deref` and `model.navigate`: two work units and no result unit; `incomplete { limit_kind: work_units, limit: 1, consumed: 1, next_charge: 1, charge_point: model.navigate }`; `refused { code: dangling_reference, cause: absent-target-in-complete-population }` for `z9` after `model.deref`: one work unit; `incomplete { limit_kind: work_units, limit: 0, consumed: 0, next_charge: 1, charge_point: model.deref }` |

## Expected Results

Every positive and boundary result matches FR-153; each invalid, unsupported or exhausted mutation returns its exact typed disposition with source/provenance, emits no approximation, and preserves independent sibling results.
