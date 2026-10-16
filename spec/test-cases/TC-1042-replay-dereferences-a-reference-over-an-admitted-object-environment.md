---
id: TC-1042
title: "Replay dereferences a Reference over an admitted object environment and refuses each defect in order"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-098
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-071
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-070
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-106
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-364
    type: verifies
---
# TC-1042: Replay dereferences a Reference over an admitted object environment and refuses each defect in order

## Description

Verify that a replay request may name a snapshot document as its object
environment, that the executor admits it by FR-106 and resolves each
reference argument by FR-364's `resolve` before the call, that a function then
dereferences a held reference and completes, that a request with no
environment still completes no value, and that each defect refuses with the
code FR-106 or FR-364 already names, in the order FR-098 states. Verify the
reference witness form and its decode refusals.

Scope: FR-098-AC-14, FR-098-AC-15, FR-098-AC-16, FR-098-AC-17, FR-098-AC-18,
FR-098-AC-19, FR-071-AC-12, FR-070-AC-16, FR-070-AC-17.

## Test Procedure

Fixture **graph**: domain package `ix://test/graph`, selected with `M::Node`,
`M::Hub` and `M::Lone` resolving to its object types, all concrete. `Node` has
`items: Sequence<Int[0, 9]>[0, 3]` and the optional field
`next: Reference<Node>`. `Hub` specializes `Node` and adds no field. `Lone` is
its own supertype component and has `n: Int[0, 9]`. One population `G`
(`ix://test/graph/G`), extent `closed`, member types `[Node, Lone]`; `Hub`
conforms to `Node`, so `G` covers it. `Node` and `Hub` share the universe `U`,
and `Lone` has the universe `V`. `TN`, `TH` and `TL` are the 64-hex node ids of
`Node`, `Hub` and `Lone`. The unit declares:

- `function fa using v(n: Reference<M::Node>): Boolean pure { forall(x in deref(n).items: x < 5) }`
- `function fb using v(n: Reference<M::Node>): Boolean pure { n = n }`
- `function fc using v(x: Int[0, 9]): Boolean pure { x < 5 }`
- `function fe using v(p: Sequence<Reference<M::Node>>[0, 2]): Boolean pure { forall(r in p: forall(x in deref(r).items: x < 5)) }`

The proved verdict of every request is `violation`. `P` is the parameter node
id of the function under replay. A reference to the object with key `k` and
type `T` is `ref(k, T)`:
`{"identity":"<hex of the UTF-8 bytes of k>","object_type":"<T>","type":"reference","universe":"<U, or V for TL>"}`
(JCS member order). The identity hex of `a` is `61`, of `b` `62`, of `h` `68`,
of `z` `7a`, of `n8` `6e38` and of `n9` `6e39`. A `Witness` entry is
`P=<value text>`, and the value texts below hold none of `%`, `;`, `<` or `>`.

Environment document **S1**, format `quire.state.snapshot/v1`, observation
`current`, anchor `{"kind":"handler","name":"run"}`, the package's model
selection, and the one population `G` listed `"complete": true` with the
objects (each lists every declared field, `next` absent unless stated):

| key | type | `items` | `n` |
| --- | --- | --- | --- |
| `a` | `Node` | `[3, 7]` | |
| `b` | `Node` | `[1, 2]` | |
| `h` | `Hub` | `[9]` | |
| `z` | `Lone` | | `4` |

Variants of S1, each with its own digest in the request's `object_environment`
and its document under that digest in the byte provision unless a row says
otherwise: **S1-anchor** (anchor name `other`); **S1-pre** (observation
`pre`, no `anchor`); **S1-model** (a `model` digest that is none of the
package's selections); **S1-dup** (a second object `a` of type `Node` after
`z`); **S1-shape** (`a`'s `items` is `{"integer":"3"}`); **S1-range** (`a`'s
`items` is `{"sequence":[{"integer":"12"}]}`); **S1-gone** (`a`'s `next` is
`{"present":{"reference":{"population":"ix://test/graph/G","key":"gone"}}}`);
**S2** (`G` listed `"complete": false`).

The manifest of every document has one row, `G`, covering `Node`, `Hub` and
`Lone`, complete exactly when the document lists `G` `"complete": true`.
Tag the tests `#[trace("TC-1042", "FR-098-AC-n")]` with the AC each backs.

### Request member and reference witness decode

| # | Step | Expected |
| --- | --- | --- |
| 1 | a request whose `object_environment` is `{"digest": <S1's sha256-jcs digest>, "identity": {...}}`, with no byte-provision entry under that digest | constructs; the member serializes and reads back equal |
| 2 | `object_environment` is the string `"S1"` | `invalid_runtime_input`/`wrong-value-kind` naming `object_environment` |
| 3 | `object_environment` is `{"identity": {...}}` | `missing-member` naming `object_environment.digest` |
| 4 | `object_environment` is `{"digest": ..., "identity": {...}, "role": "current"}` | `unknown-member` naming `object_environment.role` |
| 5 | `object_environment.digest` in the `quire.source.bytes/v1` domain; then a `sha256-jcs` digest of 63 hex digits | `invalid-value` naming `object_environment.digest`, twice |
| 6 | decode the `Canonical` entry `P={"identity":"61","object_type":"<64 digits 2>","type":"reference","universe":"<64 digits 1>"}`; then the same with `"identity":""` | one `WitnessValue::Reference` with universe 64 digits `1`, object type 64 digits `2` and identity bytes `61`; then one with empty identity bytes (decode reads the form only) |
| 7 | decode each of: `universe` of 63 digits; `object_type` of 63 digits `2` and `A`; `"identity":"616"`; the members written `{"type":"reference","universe":...,"object_type":...,"identity":...}`; no `identity`; `{"extra":"y","identity":"61",...}`; `"universe":1` | `DecodeRefusal::Malformed` naming the parameter, seven times |

### Replay over the environment

| # | Step | Expected |
| --- | --- | --- |
| 8 | `fa`, `Input` `n` = `ref(a, TN)`, environment S1 | `reproduced-without-witness`; the call completes `false`: `deref(n).items` is `[3, 7]` and `7 < 5` is false |
| 9 | row 8 from a `Witness` entry `P=ref(a, TN)` | `reproduced-with-evaluated-witness`, evaluated `false` |
| 10 | `fa`, `ref(b, TN)`, S1 | completes `true` (`[1, 2]`); `inconclusive`, cause `Verdicts` holding `violation` and `success` |
| 11 | `fa`, `ref(h, TH)`, S1 | `h`'s `object_type` `TH` conforms to `Node`; completes `false` (`9 < 5` is false); `reproduced-without-witness` |
| 12 | `fa`, `ref(a, TN)`, **S1-anchor** | the same result as row 8: the anchor is not compared |
| 13 | `fe`, `p` = `{"elements":[ref(a, TN), ref(b, TN)],"type":"sequence"}`, S1 | completes `false` (the element `a` has `7`); `reproduced-without-witness` |
| 14 | `fa`, `ref(a, TN)`, no `object_environment` | `inconclusive`, cause `NoValue`, on `unknown_required_feature`/`unsupported-feature` with construct `dereference` |
| 15 | `fb`, `ref(a, TN)`, no `object_environment`; then with S1 | completes `true` both times; `inconclusive`, cause `Verdicts` |
| 16 | `fa`, `n` the Witness entry `P={"fields":[{"name":"items","value":{"elements":[],"type":"sequence"}},{"name":"next","value":null}],"name":"<TN>","type":"record"}`, S1 | the entry decodes; `WrongValueKind` naming position 0 before the call: a record is not a `Reference`'s kind |

### Reference conversion, before the document is read

Rows 17 to 19 give `object_environment` a digest with no byte-provision
entry, so a refusal other than `unavailable_observation` shows the document was
never read.

| # | Step | Expected |
| --- | --- | --- |
| 17 | `fa`, `ref(z, TL)` | `WrongValueKind` naming position 0: `Lone` neither is `Node` nor conforms to it |
| 18 | `fa`, a reference whose `object_type` is 64 digits `0` | `WrongValueKind` naming position 0: no object type of the model has that id |
| 19 | `fa`, `ref(a, TN)` with `"identity":""`; then with `"identity":"ff"` | `WrongValueKind` naming position 0, twice: no kernel object identity is empty or not UTF-8 |

### Unused environment

| # | Step | Expected |
| --- | --- | --- |
| 20 | `fc`, `Input` `x` = integer 7, `object_environment` with a digest that has no byte-provision entry | `UnusedObjectEnvironment`: `invalid_runtime_input`/`unknown-member` naming `object_environment`, not `unavailable_observation` |
| 21 | `fc`, `Input` `x` = `WitnessValue::Boolean(true)`, S1 in the provision | the row 20 refusal, not `WrongValueKind`: the check precedes conversion |

### Environment admission

All rows are `fa` over `ref(a, TN)` unless stated, with the refusal
`ReplayRefusal::ObjectEnvironment` holding FR-106's record, and no call.

| # | Step | Expected |
| --- | --- | --- |
| 22 | S1's digest, no byte-provision entry | `Incomplete`, `unavailable_observation`/`missing-required-artifact` (check 1) |
| 23 | **S1-pre** | `wrong_snapshot`/`wrong-observation` (check 3) |
| 24 | **S1-model** | `invalid_model_binding`/`wrong-model-selection` (check 4) |
| 25 | **S1-dup**, with `ref(n9, TN)` as the argument | `invalid_runtime_input`/`conflicting-identity` at the second `a` of `G` (check 6.3), not the dangling `n9` |
| 26 | **S1-shape**; then **S1-range** | `invalid_runtime_input`/`wrong-value-kind` naming `a` and `items`; then `invalid-value` naming `a` and `items` (check 6.5) |
| 27 | **S1-gone** | `dangling_reference`/`absent-target-in-complete-population` naming `gone` and `G` (check 8) |
| 28 | **S2** | `Incomplete`, `incomplete_population`/`incomplete-scope` naming `G`: `G` holds `a` and is not complete (check 7) |

### Reference resolution

Each refusal is `ReplayRefusal::ObjectEnvironment`, names position 0 and the
reference, and makes no call.

| # | Step | Expected |
| --- | --- | --- |
| 29 | `fa`, `ref(n9, TN)`, S1 | `dangling_reference`/`absent-target-in-complete-population` for `n9`, which `G` does not list: the manifest is closed and `G`, the one row covering `Node`, is complete (FR-364 rule 5) |
| 30 | `fa`, `ref(a, TN)` with universe `V`; then `ref(a, TH)`, both S1 | `dangling_reference`/`absent-target-in-complete-population`, twice: the held `a` has the key (`U`, `TN`, `a`), and a key differing in universe or type is another reference; `G` covers `Hub` |
| 31 | `fa`, `ref(n9, TN)`, **S2** | `Incomplete`, `incomplete_population`/`incomplete-scope` for `n9`: no held object names a required population, so admission succeeds, and the covering row `G` is not complete (`undecidable`, FR-364 rule 4) |
| 32 | `fe`, `p` = `[ref(n9, TN), ref(n8, TN)]`, S1 | `dangling_reference` naming position 0 and `n9`, not `n8` |
| 33 | `fe`, `p` = `[ref(a, TN), ref(n9, TN)]`, S1 | `dangling_reference` naming position 0 and `n9`; `a` is present and not named |

## Expected Results

A request with an `object_environment` names one snapshot document; the
executor admits it by FR-106's checks 1, 3 (role), 4, 6, 7 and 8 and refuses
with FR-106's records, then resolves each reference argument by FR-364's
`resolve` and refuses a `dangling` one as `dangling_reference` and an
`undecidable` one as `incomplete-scope`, then calls the function over the
admitted environment. A held reference dereferences to its stored field and
the call completes; with no environment the same call completes no value. A
reference is held only by its whole key, and a conversion refusal or an unused
environment is decided before the document is read.
