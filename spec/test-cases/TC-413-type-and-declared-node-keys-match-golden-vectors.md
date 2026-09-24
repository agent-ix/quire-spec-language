---
id: TC-413
title: "Type and declared record nodes key to the structural-node golden vectors, scoped only by owner"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-092
    type: verifies
---
# TC-413: Type and declared record nodes key to the structural-node golden vectors, scoped only by owner

## Description

Verify that `check` builds each `Value`-family type node in the FR-092 shape
and keys it by the `quire.structural-node/v1` preimage, byte for byte, that
builtin and anonymous type nodes carry no owner while a declared record
carries its unit's owner, that an alias adds no node, that the preimage walk
is bounded, and that nominal enum nodes keep QSpec's own preimage.

This catches a preimage that drifts from the authored bytes, an owner leaking
into an anonymous type (which splits `Int[0, 9]` per package), an owner
missing from a declared record (which merges two packages' `Point`), and a
catch-all arm that silently keys a new type form.

Scope: FR-092-AC-1, FR-092-AC-2, FR-092-AC-3, FR-092-AC-7, FR-092-AC-8,
FR-092-AC-9, FR-092-CON-2.

## Test Procedure

Every fixture unit starts with the complete-V1 header and one profile
selection whose alias is `v`.

1. Build, through `check`, the type nodes of `Boolean`, `Integer`, the text
   scalar base, `Int[0, 9]`, `Option<Int[0, 9]>`, `Sequence<Int[0, 9]>`,
   `Sequence<Int[0, 9]>[0, 5]` and `Text[0, 64; nfc]`. Read each node's
   preimage bytes and key.
2. Check a unit with `function g1 using v(x: Int[0, 9]): Boolean pure { true }`
   under owner (`a`, `u`) and again under (`a`, `w`). Read the key of `x`'s
   type node each time.
3. Check `record Point { x: Int[0, 9]; y: Int[0, 9]; }` under (`a`, `u`)
   twice and under (`a`, `w`) once. Read `Point`'s key and preimage bytes.
4. Check `type Digit = Int[0, 9];` with
   `function g using v(x: Digit): Boolean pure { true }`. Read the semantic
   type of `x`'s parameter node, and list every node's `declaration`.
5. With the depth limit set to 4, check a parameter typed with four nested
   `Option`s around `Boolean`, and one with five. Check the recursive `f` and
   `g` of FR-092-AC-7 in one unit.
6. Key QSpec's `enum-status` and `enum-status-ready` preimages from
   `node-identity-vectors.json`, read at run time from `QSPEC_DIR` (the opt-in
   `make conformance` gate).
7. Build the type nodes of `Rational[-9, 9; 1, 9]`,
   `Decimal[-100000, 100000; 2, 2; nearest-even]`,
   `tuple Pair(Int[0, 9], Int[0, 9]);` and the two `record Opt` declarations
   of FR-092-AC-9 under (`a`, `u`), and the literal nodes of `rational(1, 2)`
   and `rational(2, 4)` typed `Rational[-9, 9; 1, 9]`.
8. Scan the preimage-selection and type-node `match` expressions for a `_`
   arm.

Tag the tests `#[trace("FR-092-AC-n", "TC-413")]` with the AC each backs.

## Expected Results

- Step 1: the bytes and keys equal FR-092 vectors T1 to T8.
- Step 2: both keys equal T4.
- Step 3: both (`a`, `u`) keys equal D1 and the (`a`, `w`) key equals D2. D1's
  preimage has `owner` and `declaration`; T4's has neither member.
- Step 4: the semantic type is T4, and no node's `declaration` is `Digit`.
- Step 5: four `Option`s are keyed; five refuse with
  `resource_exhausted`/`insufficient-next-charge` naming the depth limit. The
  `f`/`g` unit refuses with `unknown_required_feature`/`unsupported-feature`
  naming both functions' in-group node regions.
- Step 6: each key equals the recorded `sha256`, and each preimage's
  `version` is its nominal one.
- Step 7: T10 and T9, T12 and T11, D5, D3, D4 (differing from D3), and L3
  for both literals, spelled `"1/2"`.
- Step 8: no `_` arm.

## Status

Planned; QSL-156 A4b. Step 3's owner needs QSL-159's source authority.
