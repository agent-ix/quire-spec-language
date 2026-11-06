---
id: TC-413
title: "Type and declared nodes carry structural ids and checked shapes"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-092
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-093
    type: verifies
---
# TC-413: Type and declared nodes carry structural ids and checked shapes

## Description

Verify FR-092-AC-1/2/3/7/8/9/11/12 and FR-092-CON-2 against the
QSpec FR-322 structural node identity contract. Read QSpec's current
conformance vectors from its checkout; no QSpec fixture is copied here.

## Test Procedure

1. Check the scalar, bounded, option, collection, record and tuple examples
   of FR-092's Type nodes table. Inspect each tag, semantic form, semantic
   type, body, declaration, owner and structural `{package, path, ordinal}`.
2. Compile `Int[0,9]` from two units of one package. In separate compile
   requests, compile `Point` from unit A, from unit B, and twice from unit A;
   never place duplicate `Point` declarations in one package. Compare ids
   componentwise, and inspect the declared node's source owner and
   declaration source-map region.
3. Check `type Digit = Int[0,9]` and a parameter typed `Digit`; count type
   nodes and inspect the parameter's semantic type.
4. Run the nested `Option` and optional-record-chain cases in FR-092-AC-7
   on the specified small stacks, then repeat on larger stacks. Check
   separately named recursive `f` and `g` and inspect their structural ids.
5. Check enum `Status` and its member against QSpec FR-322's current
   structural shape and source owner rules.
6. Check the rational/decimal, tuple and optional-field examples of
   FR-092-AC-9, including `rational(1,2)` and `rational(2,4)`.
7. Check recursive `f`, `List`, `Tree`, `ping` and `pong`, including reversed
   declaration order; inspect group membership and references. Check that a
   caller handle supplied to composite construction never becomes `Point`'s
   node id.
8. Inspect node-form and type-form matches for catch-all arms.
9. Emit a unit holding `Quantity<m, Decimal[-100, 100; 2, 2; exact]>`,
   `Quantity<m, Int[-2, 2]>` and `Quantity<m, Int[-2, 2; nearest-even]>`
   parameters (FR-092-AC-27). Inspect each carried node's and the decimal
   carrier's structural path against QSpec's `dec_cents` and `q_dec`
   vectors, read the package with the IR v2 reader, then truncate a carried
   node's path and read again.
10. For FR-092-AC-30, emit bounded nominal quantities using QSpec BQ-01
    through BQ-04; inspect the complete body, self semantic type, canonical
    path and exact sorted dependencies. Compare alias/equivalent-endpoint,
    changed-endpoint and changed-unit identities. Verify missing endpoints
    and compound units do not produce this form.
11. For FR-093-AC-61, run its add, convert and less-than values and refusal
    controls through QSL checking, lowering and evaluation. Inspect operation
    identities and result types; keep the TC-440 bare-unit exact-id controls.
12. Read emitted bounded packages with the native IR reader and mutate their
    endpoints/path as QSpec BQ-05/BQ-06 require.

## Expected Results

Every node has the type and body shape FR-092 states and a valid FR-322
structural id. Equal ids compare equal componentwise; distinct declared
paths remain distinct. Source ownership and regions follow their actual
units. The alias adds no node. Deep cases terminate without stack-dependent
ids; recursive references retain their targets and group membership.
Optional-field syntax distinguishes the two record shapes and equivalent
rational values reduce equally. No catch-all match arm is present. No
preimage byte, digest, or obsolete T/D/G/L/E/P vector is an oracle.

Step 9: each carried path holds unit, carrier, `"rounding"`, text type and mode in
that order, the decimal carrier repeats its equal scale text, the reader admits
the package and refuses the truncated path (FR-092-AC-27).

Steps 10–12 satisfy FR-092-AC-30 and FR-093-AC-61, using QSpec's published
bounded-quantity vectors as the authority.

## Implementation status

The enabled QSL-side tests use the public typed declaration API for bounded
aliases and function forms, a genuinely source-assembled nominal unit graph,
and the production checker, lowering and evaluator with real quantity
arguments. They do not claim a source grammar for bounded quantity types.
Type-key, addition, comparison and conversion into and out of a bounded
quantity are enabled. Conversion into a bounded quantity checks the exact
converted magnitude against both inclusive endpoints and refuses with
quire-exact's `BoundedQuantityOutOfDomain`, carrying the target unit and
interval (QSpec FR-322-AC-384, BQ-09).
