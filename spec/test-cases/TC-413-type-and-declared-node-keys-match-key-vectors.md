---
id: TC-413
title: "Type and declared nodes carry structural ids and checked shapes"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-092
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

## Expected Results

Every node has the type and body shape FR-092 states and a valid FR-322
structural id. Equal ids compare equal componentwise; distinct declared
paths remain distinct. Source ownership and regions follow their actual
units. The alias adds no node. Deep cases terminate without stack-dependent
ids; recursive references retain their targets and group membership.
Optional-field syntax distinguishes the two record shapes and equivalent
rational values reduce equally. No catch-all match arm is present. No
preimage byte, digest, or obsolete T/D/G/L/E/P vector is an oracle.
