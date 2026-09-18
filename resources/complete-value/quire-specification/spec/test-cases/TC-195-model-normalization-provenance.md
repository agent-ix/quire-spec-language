---
id: TC-195
title: "Model normalization provenance"
type: TC
relationships:
  - target: ix://agent-ix/quire-specification/FR-150
    type: verifies
---

# TC-195: Model normalization provenance

## Description

Normalize inherited/redefined declarations and mutate identity, digest and rule-order inputs.

## Test Procedure

Use the exact version-locked subject and inputs named by FR-150. Execute its positive case, each declared boundary, and each explicit refusal/non-conclusive mutation through the real local Rust boundary; compare the complete typed result and artifact accounting with the criterion.

Select `quire.model.complete/v1` at revision `1-draft.1` by its exact
DefinitionRefs in
[`complete-model-lock.json`](../../proposals/quire-v1/definitions/complete-model-lock.json),
which also pins the `quire.value.complete/v1` lock. Every fixture is a producer
interface `1.3.0` bundle with header `generalizationClosure: closed`,
`subsettingClosure: closed` and `redefinitionClosure: closed` unless a row says
otherwise. Every producer key has authority `filament-core-data`,
revision `{namespace: filament-core-data/producer-object-revision-1, value: 1}`
and digest domain `filament-canonical-json-1` whose `sha256` is the SHA-256 of
the identity's UTF-8 bytes; these digests are opaque test inputs. Effective
identities are written as the first eight hex digits of the full digests in
[`model-effective-declaration-vectors.json`](../../proposals/checked-package-v2/model-effective-declaration-vectors.json).
Unless a row states `ModelNormalizationLimitsV1`, run it under limits large
enough that no charge is denied.

Fixtures:

- **F1** (bundle `bundle.n01`): object type exports `model.A` and `model.B`;
  field member `model.A.x` of `model.A`, typed `model.A` with multiplicity
  `{lower: 0, upper: 1, ordered: false, unique: true}`; generalization record
  `model.gen.B-A` (`B` specific, `A` general). Four records.
- **F2** (bundle `bundle.n02`): F1's records plus object type `model.C` and
  `model.D` and generalization records `model.gen.C-A`, `model.gen.D-B` and
  `model.gen.D-C`. Nine records.
- **Source S1**: `language "ix:native" edition "1-draft";`, one profile with
  alias `V`, `model
  M = "filament-core-data:bundle.n02" version "1" digest "<F2 digest>";`.

| Vector | Input | Expected result/disposition |
| --- | --- | --- |
| N01 | Normalize F1; then under `ModelNormalizationLimitsV1 { producer_records: 4, derivation_facts: 5, effective_declarations: 4, dispatch_candidates: 0, hashed_bytes: 9989, work_units: 20 }`; then with `work_units: 19`; then with `hashed_bytes: 9988`; then with `derivation_facts: 4` and every other counter as in the exact run | effective types `A` = `f1cc59cd` (one qualify fact) and `B` = `b9953c43` (qualify fact, then inherit fact with inputs `[model.gen.B-A, model.A]`), effective members `(A, A.x)` = `4c06822f` and `(B, A.x)` = `e8a29d61` (one inherit fact with inputs `[model.gen.B-A, model.A.x]`), effective view `9ae1232a` and object universe `0873083c` with root types `[f1cc59cd]`; charges, in order, are four `normalize.record`; five `normalize.fact` (phase 2 qualify `A`, `B`, `(A, A.x)`; phase 3 `normalize.cycle-check` with `L = 1` then inherit `B`; inherit `(B, A.x)`); four `normalize.declaration`, each followed by its `normalize.hash` (JCS lengths 739, 1380, 864 and 1135); then `normalize.hash` of the universe (591) and of the view (5280): twenty work units and 9989 hashed bytes; the exact run completes identically; `incomplete { limit_kind: work_units, limit: 19, consumed: 19, next_charge: 1, charge_point: normalize.hash }` at the view hash; `incomplete { limit_kind: hashed_bytes, limit: 9988, consumed: 4709, next_charge: 5280, charge_point: normalize.hash }` at the view hash; `incomplete { limit_kind: derivation_facts, limit: 4, consumed: 4, next_charge: 5, charge_point: normalize.fact }` at the fact for `(B, A.x)`; neither incomplete run exposes an effective view, and the checker reports it as `resource_exhausted` with cause `insufficient-next-charge` at stage `model-normalization` |
| N02 | Normalize F2 | eight effective declarations: types `A` = `f1cc59cd`, `B` = `b9953c43`, `C` = `7c28ad04`, `D` = `51796212` (qualify plus four inherit facts, one per path prefix `[gen.D-B, B]`, `[gen.D-B, gen.B-A, A]`, `[gen.D-C, C]`, `[gen.D-C, gen.C-A, A]` in that order); members `(A, A.x)` = `4c06822f`, `(B, A.x)` = `e8a29d61`, `(C, A.x)` = `118a8e09` and exactly one `(D, A.x)` = `13a71b44` retaining both paths `[gen.D-B, gen.B-A, A.x]` and `[gen.D-C, gen.C-A, A.x]`; view `d3a49797`; universe `ed29d710`; nine records, fifteen facts (five qualify, ten inherit), six `normalize.cycle-check` charges before the six phase 3 type facts (`L` = 1 for `B`, 1 for `C`, then 1, 2, 1 and 2 for `D`'s four paths: eight work units), eight declarations and ten hashes (declarations 12798 bytes, universe 591, view 14580): fifty work units and 27969 hashed bytes. `A` and `(A, A.x)` equal their F1 identities because an effective declaration identity binds no ModelSelection, while the views and universes differ |
| N03 | F2 plus field members `model.B.y` and `model.C.y`, both typed `model.A` with multiplicity `{0,1,false,true}`; then S1 plus `invariant I using V on M::D at current { present(self.y) }` | normalization admits twelve effective declarations, N02's eight plus `(B, B.y)`, `(C, C.y)` and the distinct, equal-shaped `(D, B.y)` and `(D, C.y)`; checking the invariant is `refused { code: ambiguous_declaration, cause: ambiguous-name }` at `self.y` listing both effective members and loci |
| N04 | F1 with the `revision` member removed from `model.gen.B-A`; then F1 with a `variant` type export `model.V` referenced as `M::V` in a record field of S1 | `refused { code: invalid_model_binding, cause: wrong-model-selection }` naming `model.gen.B-A` and no effective view; normalization admits `model.V` as an effective type, and checking refuses `refused { code: unsupported_construct, cause: declaration-form }` at `M::V` |
| N05 | F1 with `model.A.x`'s digest domain replaced by `quire-native-bytes-1`, digest bytes unchanged | `refused { code: stale_dependency, cause: digest-domain-mismatch }` with expected `filament-canonical-json-1` and actual `quire-native-bytes-1`; no effective view |
| N06 | F2 plus field members `model.B.x2` and `model.C.x3` of type `model.A` and multiplicity `{0,1,false,true}`, and redefinition records `model.redef.B` (`B`, `B.x2` redefines `A.x`) and `model.redef.C` (`C`, `C.x3` redefines `A.x`); then additionally field `model.D.x4` of the same type and multiplicity with record `model.redef.D` (`D`, `D.x4` redefines `A.x`) | `refused { code: invalid_model_binding, cause: derivation-conflict }` at `D` with paths `[model.gen.D-B, model.redef.B, model.A.x]` and `[model.gen.D-C, model.redef.C, model.A.x]`, and no effective view; admitted: `D` is a proper descendant of `B` and `C`, so only the most-derived owner's redefining feature is exposed. `(D, D.x4)` is exposed with its qualify fact and one redefine fact with inputs `[model.redef.D, model.A.x]`; `(D, A.x)` is retained but hidden, with redefine facts `[model.gen.D-B, model.redef.B, model.A.x]`, `[model.gen.D-C, model.redef.C, model.A.x]` and `[model.redef.D, model.A.x]` in that order; `(D, B.x2)` and `(D, C.x3)` keep their inherit facts and their redefine facts (`[model.gen.D-B, model.redef.B, model.A.x]` and `[model.gen.D-C, model.redef.C, model.A.x]` respectively) and are hidden |
| N07 | F2 with its records supplied in reverse order | the same eight identities and view `d3a49797` as N02 |
| N08 | F1 declared as producer interface `1.2.0` (no generalization record and no `1.3.0` inline members); then F1 declared as `1.4.0` | after three `normalize.record` and ten `normalize.unsupplied-item` charges, ten `refused { code: invalid_model_binding, cause: unsupplied-producer-record }` with `required: 1.3.0`, `supplied: 1.2.0`, in this order: `subtype-closure`, `subsetting-closure` and `redefinition-closure` for `bundle.n01`; `generalization` for `model.A`, then `model.B`; `redefinition` for `model.A.x`; `subsetting` for `model.A.x`; `interface-signature` for `model.A`, then `model.B`; `typed-multiplicity` for `model.A.x`; and no effective view; `refused { code: unknown_wire, cause: unsupported-wire }` before any charge |
| N09 | F1 with the ModelSelection key recomputed; then the N01 identities compared with every F1 producer key and with the I04 node key of the `model_correspondence` node | key `(authority filament-core-data, export (bundle.n01, revision 1, filament-canonical-json-1 digest), contract_version (1.3.0, filament-core-data/producer-interface/1.3.0))`, exactly the `model_selection` of the `n01-view` vector; no effective, view or universe identity equals any producer key or I04 node key, because their domains differ |
| N10 | Each mutation in `invalid_mutations` of the vectors file applied to its base preimage | `stale-digest` refuses as a stale key; `cross-domain-producer-digest` and `owner-as-producer-key` are refused by the schema; `unsorted-derivation`, `duplicate-path` and `unsorted-view` are refused by the semantic check; none exposes its retained digest as a valid identity |

## Expected Results

Every positive and boundary result matches FR-150; each invalid, unsupported or exhausted mutation returns its exact typed disposition with source/provenance, emits no approximation, and preserves independent sibling results.
