---
id: TC-414
title: "Parameter, literal and function nodes key to the golden vectors, and a function key carries its owner"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-092
    type: verifies
---
# TC-414: Parameter, literal and function nodes key to the golden vectors, and a function key carries its owner

## Description

Verify the FR-092 parameter and function node shapes and their keys: that a
parameter node is keyed by name, type and level, that a function lists every
parameter in order, that a function's body holds references only and its key
carries its owner, and that an application node's key carries no owner.

This catches an arity lost when a parameter is never read, a parameter key
that embeds its function (a cycle), an integer literal spelled as a JSON
number, and a function node keyed by the application-node preimage.

Scope: FR-092-AC-4, FR-092-AC-5, FR-092-AC-6.

## Test Procedure

Every fixture unit starts with the complete-V1 header and one profile
selection whose alias is `v`, and is checked under owner (`a`, `u`).

1. Check `function both using v(a: Boolean, b: Boolean): Boolean pure { a and b }`,
   `function f using v(): Boolean pure { true }`,
   `function h using v(a: Boolean): Boolean pure { let y = a in y }` and
   `function k using v(n: Boolean): Integer pure { 7 }`. Read the keys and
   preimage bytes of the parameter nodes of `a`, `b` and `y`, the literal
   nodes of `true` and `7`, and the function nodes of `both` and `f`.
2. Check `function unused using v(a: Boolean, b: Boolean): Boolean pure { a }`
   and `function both2 using v(a: Boolean, b: Boolean): Boolean pure { a and b }`
   in one unit with `both`. Read `unused`'s `parameters` binding and the
   parameter nodes `both2` references.
3. For every function node of steps 1 and 2, walk its body for an
   `application` term, and read its preimage's `version` and `owner`. Read the
   preimage of the node of `a and b`.

Tag the tests `#[trace("FR-092-AC-n", "TC-414")]` with the AC each backs.

## Expected Results

- Step 1: `a` is P1, `b` P2, `y` P3, `true` L1, `7` L2 (value spelled
  `"7"`), `both` F2 and `f` F1, bytes and keys.
- Step 2: `unused`'s `parameters` lists P1 then P2; `both2` references P1
  and P2.
- Step 3: no function body holds an application; every function preimage's
  `version` is `quire.structural-node/v1` with `owner` (`a`, `u`). The
  `a and b` preimage's `version` is `quire.application-node/v1`, it has no
  `owner` member, and its key is E1.

## Status

Planned; QSL-156 A4b.
