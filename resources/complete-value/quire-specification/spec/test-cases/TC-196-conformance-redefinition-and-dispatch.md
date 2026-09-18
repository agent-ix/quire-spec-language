---
id: TC-196
title: "Conformance redefinition and dispatch"
type: TC
relationships:
  - target: ix://agent-ix/quire-specification/FR-151
    type: verifies
---

# TC-196: Conformance redefinition and dispatch

## Description

Resolve valid specialization/dispatch and reject cycles, weakening, ambiguity and incomplete closure.

## Test Procedure

Use the exact version-locked subject and inputs named by FR-151. Execute its positive case, each declared boundary, and each explicit refusal/non-conclusive mutation through the real local Rust boundary; compare the complete typed result and artifact accounting with the criterion.

Select `quire.model.complete/v1` at revision `1-draft.1` by its exact
DefinitionRefs in
[`complete-model-lock.json`](../../proposals/quire-v1/definitions/complete-model-lock.json).
Producer keys, header closure and default limits follow TC-195. Every
multiplicity is written `{lower,upper}` with `ordered: false` and `unique:
true` unless stated. Scalar type exports `model.Count` and `model.Small` are
bound as `Int[0,9]` and `Int[0,5]`. Effective type identities are TC-195 N02's:
`D` = `51796212` < `C` = `7c28ad04` < `B` = `b9953c43` < `A` = `f1cc59cd` in
ascending digest order.

Fixtures:

- **G** (bundle `bundle.g`): F2's types and generalizations; `model.Count`;
  operation member `model.A.size` with signature `{receiverTypeIdentity:
  model.A, parameters: [], result: {typeIdentity: model.Count, multiplicity:
  {1,1}}, effect: {fieldWrites: [], creates: [], deletes: []}}`; operation
  member `model.B.size` with the same signature and receiver `model.B`; and
  redefinition record `model.redef.B.size` (`B`, `B.size` redefines `A.size`).
- **S2**: TC-195 S1 (profile alias `V`, model alias `M`) importing `bundle.g`, plus `body sizeA using V on
  M::A::size() { 1 }`, `body sizeB using V on M::B::size() { 2 }` and
  `invariant I using V on M::A at current { self.size() >= 1 }`.
- **H** (bundle `bundle.h`): F1's types `A`, `B` and `model.gen.B-A`; field
  `model.A.x` typed `model.A` `{0,1}`; field `model.B.y` typed `model.A`
  `{0,1}`; operation `model.A.op` with parameters `[{parameterIdentity:
  model.A.op.p1, typeIdentity: model.A, multiplicity: {0,1}}]`, result
  `{model.B, {1,1}}` and effect `{fieldWrites: [model.A.x], creates: [model.A],
  deletes: []}`; operation `model.B.op` with record `model.redef.B.op` (`B`,
  `B.op` redefines `A.op`) and the signature each row states, or R02's signature
when a row states none.

| Vector | Input | Expected result/disposition |
| --- | --- | --- |
| R01 | Types `A`, `B` with generalization records `model.gen.A-B` and `model.gen.B-A` | exactly one `refused { code: invalid_model_binding, cause: specialization-cycle }` listing `[model.gen.A-B, model.gen.B-A]`, rotated to start at the least record `model.gen.A-B`; phase 3 charges `normalize.cycle-check` for `A`'s extensions `[gen.A-B]` (`L = 1`, then its fact) and `[gen.A-B, gen.B-A]` (`L = 2`, closing at `A`: reported, no fact), then for `B`'s extensions `[gen.B-A]` (`L = 1`, then its fact) and `[gen.B-A, gen.A-B]` (`L = 2`, closing at `B` with the same record set: charged, not reported), six cycle-check work units; no effective view |
| R02 | H with `B.op` parameters `[{p1, model.A, {0,2}}]`, result `{model.B, {1,1}}`, effect `{fieldWrites: [], creates: [model.B], deletes: []}` | admitted: parameter type equal, redefined `{0,1}` conforms to `{0,2}`, result equal, and the `model.B` create is covered by the `model.A` grant; eight `conformance.axis` charges (arity, parameter type 1, parameter multiplicity 1, result type, result multiplicity, effect, precondition, postcondition) and one effective `(B, B.op)` with its redefine fact; per-axis work units: arity 1, parameter type 1 1 (`f(A) = 1`), parameter multiplicity 1 1, result type 2 (`f(B) = 2`), result multiplicity 1, effect 2 (the one create `model.B` tested against the one grant `model.A` scans `B`'s two type derivation facts), precondition 1, postcondition 1: ten work units over the eight axis charges |
| R03 | H with `B.op` parameters `[{p1, model.B, {1,1}}]`, result `{model.A, {0,1}}`, effect `{fieldWrites: [model.B.y], creates: [], deletes: []}` | five refusals in axis order, after all eight axis charges: `ill_typed`/`variance-parameter` (index 1, expected `A` conforms to actual `B`: false); `ill_typed`/`multiplicity-narrowing` (parameter 1, `{0,1}` to `{1,1}`); `ill_typed`/`variance-result` (`A` to `B`); `ill_typed`/`multiplicity-narrowing` (result, `{0,1}` to `{1,1}`); `ill_typed`/`effect-escape` (`model.B.y`); per-axis work units: arity 1, parameter type 1 1 (`f(A) = 1`), parameter multiplicity 1 1, result type 1 (`f(A) = 1`), result multiplicity 1, effect 1 (the write `model.B.y` compared against the one redefined write `model.A.x`, with no redefinition record walked), precondition 1, postcondition 1: eight work units |
| R04 | H with `B.op` parameters `[]` and R02's result and effect | `refused { code: ill_typed, cause: type-mismatch }` on the arity axis (expected 1, actual 0); no parameter axis is charged, so six `conformance.axis` charges: arity 1, result type 2, result multiplicity 1, effect 2, precondition 1, postcondition 1: eight work units |
| R05 | Types `A`, `B ≤ A`; field `model.A.n` typed `model.A` `{1,3}` and `model.B.n` typed `model.A` `{0,5}` redefining it; then with the multiplicities swapped; then `A.n` `{0,unbounded}` and `B.n` `{0,5}`; then `A.n` `{0,5}` and `B.n` `{0,unbounded}`; then equal bounds with `B.n` `ordered: true` | `refused { code: ill_typed, cause: multiplicity-narrowing }` with expected `{1,3}` and actual `{0,5}`; admitted; admitted; `multiplicity-narrowing`; `multiplicity-narrowing` |
| R06 | Types `A`, `B ≤ A`, unrelated `C`; field `model.A.all` typed `model.A` `{0,5}`; field `model.A.some` with subsetting record (`A`, `A.some` subsets `A.all`) typed `model.A` `{0,9}`; then typed `model.C` `{0,5}`; then typed `model.B` `{0,3}`; then that admitted model with a closed population where object `a1` has `all = [a2]` and `some = [a3]`; then the same population with `closedWorld: false` | `refused { code: ill_typed, cause: multiplicity-narrowing }`; `refused { code: ill_typed, cause: subsetting-type }`; admitted; binding admission (population members `a1`, `a2` and `a3`, all of type `model.A`) `refused { code: invalid_runtime_input, cause: subsetting-violation }` naming the record, `a1` and `a3`, after three `binding.member` charges and one `binding.subset-value` charge for `a1`'s one `some` value (`n = 1`): four admission work units and no evaluation charge; `incomplete_population` with cause `incomplete-scope` |
| R07 | H, with `B.op` carrying R02's signature, plus unrelated object type `model.C` with field `model.C.w` typed `model.A` `{0,1}`, field `model.A.x2` typed `model.A` `{0,1}`, field `model.B.z` typed `model.A` `{0,1}` and redefinition record (`B`, `B.z` redefines `model.C.w`); then, instead, two redefinition records naming `B.z` with targets `model.A.x` and `model.A.x2` | `refused { code: invalid_model_binding, cause: redefinition-target }` with zero inherited targets; the same refusal listing both records and both targets |
| R08 | Types `A`, `B ≤ A`, `model.Count`, `model.Small`; field `model.A.x` typed `model.A` `{0,1}`; operation `model.A.set` with no parameters, no result and effect `{fieldWrites: [model.A.x, model.A.c], creates: [], deletes: []}`; field `model.A.c` typed `model.Count` `{1,1}`. Mutations: (a) `model.B.xb` typed `model.A` `{1,1}` redefines `A.x`; (b) (a) plus `model.B.set` redefining `A.set` with the same signature and `post QB using V on M::B::set { present(self.xb) }`; (c) `model.B.xr` typed `model.B` `{0,1}` redefines `A.x`; (d) `model.B.cs` typed `model.Small` `{1,1}` redefines `A.c`; (e) (d) plus `model.B.set` with `post QC using V on M::B::set { self.cs <= 5 }`; (f) (d) plus `model.B.set` with `post QD using V on M::B::set { self.cs <= 6 }` | (a) `refused { code: undefined_expression, cause: unproved-refinement }` for `B.xb`, operation `A.set`, obligation `field-presence`; (b) admitted, because `B`'s exposed `set` has effective postcondition `QB` conjoined with `A.set`'s absent (true) postcondition, and `present(self.xb)` over the FR-146 stable path `self.xb`, read at the redefined parent member `A.x` (`model.A` `{0,1}`), yields the presence fact; (c) the same refusal with obligation `no-proof-form`; (d) the same refusal with obligation `field-domain`, because the exposed `A.set` has no establishing fact; (e) admitted: `self.cs` is read at the redefined parent member `A.c` (`Count`, interval `[0,9]`), and `self.cs <= 5` gives the interval fact `[0,5]`, contained in `Small`'s `[0,5]`; (f) `refused { code: undefined_expression, cause: unproved-refinement }` for `B.cs`, operation `B.set`, obligation `field-domain`, because the fact `[0,6]` is not contained in `[0,5]` |
| D01 | G and S2; link; then under `ModelNormalizationLimitsV1` with `dispatch_candidates: 8`, then `7`; evaluate `I` with `self` = object `d1` of type `D` under `ScalarLimitsV1` with every counter unlimited except `work_units: 6`, then `5`, then `1`; evaluate `I` for `c1` of type `C` | for `(A, A.size)`, per subtype `D`, `C`, `B`, `A` in that order: one `dispatch.subtype`, then one `dispatch.candidate` for each of `A.size` and `B.size` (work `f` = 5 for `D`, 2 for `C`, 2 for `B`, 1 for `A`), then for `D` and `B`, which each have two applicable candidates, one `dispatch.dominance` of three work units (`f(A) + f(B) = 1 + 2`, one per derivation fact scanned by the two proper-descendant tests): eight `dispatch.candidate` charges and 30 dispatch work units; linked table `D -> B.size`, `C -> A.size`, `B -> B.size`, `A -> A.size`; the `8` run links identically; `incomplete { limit_kind: dispatch_candidates, limit: 7, consumed: 7, next_charge: 8, charge_point: dispatch.candidate }`; `true` after `dispatch.select` (`value_occurrences` 2, two work units), `function.call`, body `2` and the three-charge `2 >= 1` ordering: six work units and one result unit; `incomplete { limit_kind: work_units, limit: 5, consumed: 5, next_charge: 1, charge_point: ordering.result-retain }`; `incomplete { limit_kind: work_units, limit: 1, consumed: 0, next_charge: 2, charge_point: dispatch.select }` with no method selected; `true` through `A.size` (`1 >= 1`), six work units |
| D02 | G plus operation members `model.C.size` and `model.D.size` with redefinition records (`C`, `C.size` redefines `A.size`) and (`D`, `D.size` redefines `A.size`), S2 plus `body sizeC using V on M::C::size() { 3 }`, and no body for `D.size`; then additionally `body sizeD using V on M::D::size() { 4 }` | normalization admits, because `D` dominates the `B` and `C` redefinitions; linking is exhaustive: all four subtypes are tested, with twelve `dispatch.candidate` charges and `dispatch.dominance` charges for `D` (`c = 3`, `2 × (f(A) + f(B) + f(C)) = 2 × (1 + 2 + 2) = 10` work units), `C` (`c = 2`, `f(A) + f(C) = 3`) and `B` (`c = 2`, `f(A) + f(B) = 3`); then, at the end of the stage, exactly one `refused { code: ambiguous_dispatch, cause: multiple-undominated }` for operation `(A, A.size)` and subtype `D`, candidates `A.size`, `B.size`, `C.size`, dominance pairs `B.size over A.size` and `C.size over A.size`, and no dispatch table; with `sizeD`, sixteen `dispatch.candidate` charges, a `D` dominance of `3 × (1 + 2 + 2 + 5) = 30` work units and `D -> D.size` |
| D03 | G and S2 without `sizeA` and `sizeB` | four `dispatch.subtype` charges and no `dispatch.candidate` charge, then four `refused { code: ambiguous_dispatch, cause: no-applicable }` for `(A, A.size)`, one each for subtypes `D`, `C`, `B`, `A` in that order |
| D04 | D01 with G's records supplied in reverse order and `sizeB` declared before `sizeA` | the same eight charges in the same order and the same linked table as D01 |
| D05 | D01 with `generalizationClosure: open` | the incomplete outcome `incomplete_population` with cause `unclosed-method-set` naming `(A, A.size)`, not a refusal; no dispatch table and no `dispatch.subtype` or `dispatch.candidate` charge |
| D06 | D01 plus `pre PA using V on M::A::size { false }` and `pre PB using V on M::B::size { false }`, evaluating `I` for `d1`; then without `PB` | the call is `undefined` with the catalogued reason `precondition-false` after `dispatch.select`, with no `function.call` and without selecting `A.size`; `true` through `B.size`, whose effective precondition is its absent (true) clause disjoined with `PA` |
| D07 | `function f using V(r: Reference<M::A>): Integer pure { if r.size() >= 1 then 1 else 0 }` over G and S2 | `refused { code: ill_typed, cause: operator-ineligible }` at `r.size()`: dispatched calls are admitted only in invariant, precondition and postcondition blocks |
| D08 | D01 plus `pre PS using V on M::A::size { self.size() >= 0 }`; link | `refused { code: invalid_package, cause: definition-cycle }` at link time for the strongly connected component `{PS}`, listing the dispatch edge `PS -> PS`: the call `self.size()` in `PS` has candidates `A.size` and `B.size`, and both effective preconditions contain `PS` (`B.size`'s absent clause disjoined with `PS`); no dispatch table |

## Expected Results

Every positive and boundary result matches FR-151; each invalid, unsupported or exhausted mutation returns its exact typed disposition with source/provenance, emits no approximation, and preserves independent sibling results.
