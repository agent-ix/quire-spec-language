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
which also pins the `quire.value.complete/v1` lock. Every fixture is a domain
package with identity `test/orders` and version `1`. A node written `X` has IR
node identity `ix://test/orders/X`, and its declaration key is
`{package: test/orders, node: ix://test/orders/X, digest_domain: sha256-jcs}`.
Object type `A` is declared by the artifact with id `A` and title `Alpha`, `B`
by id `B` titled `Beta`, `C` by id `C` titled `Gamma` and `D` by id `D` titled
`Delta`; each type definition's construct meaning is
`quire.meaning.model.object-type/v1`, and each field's span is its Properties
row. Rows N04 and N05 admit package bytes: their ModelSelection digest is the
SHA-256 of those JCS bytes, and the bytes are supplied under that digest. Every
other row enters at the normalization boundary with a fixed placeholder
ModelSelection, not the digest of real package bytes: F1 selects the SHA-256 of
the UTF-8 bytes `n01`, F2 of `n02`, and F1 as version `2` of `n01v2`. The
published vectors use these placeholder selections.
Effective identities are written as the first eight hex digits of the full
digests in
[`model-effective-declaration-vectors.json`](../../proposals/checked-package-v2/model-effective-declaration-vectors.json).
Unless a row states `ModelNormalizationLimitsV1`, run it under limits large
enough that no charge is denied.

Fixtures:

- **F1**: object types `A` and `B`, with `B.supertypes = [A]`; field `A/x` of
  `A`, typed `A`, presence `optional`, multiplicity
  `{lower: 0, upper: 1, ordered: false, unique: true}`. Three IR nodes.
- **F2**: F1's nodes plus object types `C` with `supertypes = [A]` and `D`
  with `supertypes = [B, C]`. Five IR nodes.
- **Source S1**: `language "ix:native" edition "1-draft";`, one profile with
  alias `V`, `model M = "test/orders" version "1" digest "<F2 digest>";`.

| Vector | Input | Expected result/disposition |
| --- | --- | --- |
| N01 | Normalize F1; then under `ModelNormalizationLimitsV1 { declaration_records: 3, derivation_facts: 5, effective_declarations: 4, dispatch_candidates: 0, hashed_bytes: 5311, work_units: 19 }`; then with `work_units: 18`; then with `hashed_bytes: 5310`; then with `derivation_facts: 4` and every other counter as in the exact run | effective types `A` = `3b79bb92` (one qualify fact) and `B` = `9c1ce45c` (qualify fact, then inherit fact with inputs `[A]`), effective members `(A, A/x)` = `750ec6cf` and `(B, A/x)` = `e8e6044d` (one inherit fact with inputs `[A, A/x]`), effective view `2cd5d1e7` and object universe `91320bde` with root types `[3b79bb92]`; charges, in order, are three `normalize.record` (`A`, `A/x`, `B`); five `normalize.fact` (phase 2 qualify `A`, `B`, `(A, A/x)`; phase 3 `normalize.cycle-check` with `L = 1` then inherit `B`; inherit `(B, A/x)`); four `normalize.declaration`, each followed by its `normalize.hash` (JCS lengths 375, 563, 500 and 583); then `normalize.hash` of the universe (349) and of the view (2941): nineteen work units and 5311 hashed bytes; the exact run completes identically; `incomplete { limit_kind: work_units, limit: 18, consumed: 18, next_charge: 1, charge_point: normalize.hash }` at the view hash; `incomplete { limit_kind: hashed_bytes, limit: 5310, consumed: 2370, next_charge: 2941, charge_point: normalize.hash }` at the view hash; `incomplete { limit_kind: derivation_facts, limit: 4, consumed: 4, next_charge: 5, charge_point: normalize.fact }` at the fact for `(B, A/x)`; neither incomplete run exposes an effective view, and the checker reports it as `resource_exhausted` with cause `insufficient-next-charge` at stage `model-normalization` |
| N02 | Normalize F2 | eight effective declarations: types `A` = `3b79bb92`, `B` = `9c1ce45c`, `C` = `1af4f5c7`, `D` = `97f66fc4` (qualify plus four inherit facts with inputs `[B]`, `[B, A]`, `[C]`, `[C, A]` in that order); members `(A, A/x)` = `750ec6cf`, `(B, A/x)` = `e8e6044d`, `(C, A/x)` = `495ce797` and exactly one `(D, A/x)` = `d58792c2` retaining both paths `[B, A, A/x]` and `[C, A, A/x]`; view `f9fbcf16`; universe `749e472d`; five records, fifteen facts (five qualify, ten inherit), six `normalize.cycle-check` charges before the six phase 3 type facts (`L` = 1 for `B`, 1 for `C`, then 1, 2, 1 and 2 for `D`'s four paths: eight work units), eight declarations and ten hashes (declarations 5482 bytes, universe 349, view 7022): forty-six work units and 12853 hashed bytes. `A` and `(A, A/x)` equal their F1 identities because an effective declaration identity binds no ModelSelection, while the views and universes differ |
| N03 | F2 plus fields `B/y` and `C/y`, both typed `A` with multiplicity `{0,1,false,true}`; then S1 plus `invariant I using V on M::D at current { present(self.y) }` | normalization admits twelve effective declarations, N02's eight plus `(B, B/y)`, `(C, C/y)` and the distinct, equal-shaped `(D, B/y)` and `(D, C/y)`; checking the invariant is `refused { code: ambiguous_declaration, cause: ambiguous-name }` at `self.y` listing both effective members and loci |
| N04 | F1's JCS bytes stating version `2`, supplied under their SHA-256, under a ModelSelection naming that digest and version `1`; then F1 with a variant type `V1`, from artifact `V1` whose construct meaning is `quire.meaning.model.variant-type/v1`, referenced as `M::V1` in a record field of S1 | `refused { code: invalid_model_binding, cause: wrong-model-selection }` with both versions and no effective view; normalization admits `V1` as an effective type, and checking refuses `refused { code: unsupported_construct, cause: declaration-form }` at `M::V1` |
| N05 | F1's JCS bytes under a ModelSelection whose digest is their SHA-256; then with the selection's digest domain replaced by `quire-native-bytes-1`, digest bytes unchanged; then with one byte of `A/x`'s multiplicity changed and the selection unchanged | admitted with three declarations; `refused { code: stale_dependency, cause: digest-domain-mismatch }` with expected `sha256-jcs` and actual `quire-native-bytes-1`; `refused { code: stale_dependency, cause: byte-digest-mismatch }` with the expected and actual digests; neither refusal builds a declaration |
| N06 | F2 plus fields `B/x2` and `C/x3` of type `A` and multiplicity `{0,1,false,true}`, with `B/x2.redefines = A/x` and `C/x3.redefines = A/x`; then additionally field `D/x4` of the same type and multiplicity with `D/x4.redefines = A/x` | `refused { code: invalid_model_binding, cause: derivation-conflict }` at `D` with paths `[B, B/x2, A/x]` and `[C, C/x3, A/x]`, and no effective view; admitted: `D` is a proper descendant of `B` and `C`, so only the most-derived owner's redefining member is exposed. `(D, D/x4)` is exposed with its qualify fact and one redefine fact with inputs `[D/x4, A/x]`; `(D, A/x)` is retained but hidden, with redefine facts `[B, B/x2, A/x]`, `[C, C/x3, A/x]` and `[D/x4, A/x]` in that order; `(D, B/x2)` and `(D, C/x3)` keep their inherit facts and their redefine facts (`[B, B/x2, A/x]` and `[C, C/x3, A/x]` respectively) and are hidden |
| N07 | F2 with its IR nodes supplied in reverse order | the same eight identities and view `f9fbcf16` as N02 |
| N08 | F1 with `A/x.presence` set to `maybe` and `B.supertypes` set to `[Z]` | after three `normalize.record` charges, two refusals in node order and no effective view: `refused { code: invalid_model_binding, cause: malformed-declaration }` naming `ix://test/orders/A/x`, member path `presence`, artifact `A` and the row's span; then `refused { code: missing_declaration, cause: missing-name }` naming `ix://test/orders/B`, absent node `ix://test/orders/Z`, artifact `B` and its span |
| N09 | F1 under its placeholder ModelSelection; then the N01 identities compared with the F1 selection digest and with the I04 node key of the `model_correspondence` node; then F1's nodes as version `2` under the placeholder selection `n01v2` | ModelSelection `{identity: test/orders, version: 1, digest_domain: sha256-jcs, digest: <SHA-256 of n01>}`, exactly the `model_selection` of the `n01-view` vector; no effective, view or universe identity equals the F1 selection digest or the I04 node key, because their domains differ; the version `2` run yields declarations byte-equal to `n01-view`'s, view `e6f8a8cd` and universe `3526be8a`, because only its ModelSelection differs |
| N10 | Each mutation in `invalid_mutations` of the vectors file applied to its base preimage | `absent-node` refuses as a stale key; `cross-domain-declaration-key` and `owner-as-declaration-key` are refused by the schema; `unsorted-derivation`, `duplicate-path` and `unsorted-view` are refused by the semantic check; none exposes its retained digest as a valid identity |

## Expected Results

Every positive and boundary result matches FR-150; each invalid, unsupported or exhausted mutation returns its exact typed disposition with source/provenance, emits no approximation, and preserves independent sibling results.
