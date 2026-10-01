---
id: TC-883
title: "Malformed, colliding, cyclic and unselected extensions refuse with the definitions involved"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-354
    type: verifies
---
# TC-883: Malformed, colliding, cyclic and unselected extensions refuse with the definitions involved

## Description

Verify each extension refusal names the definitions, keyword, node kind or form involved.

Scope: FR-354-AC-3, FR-354-AC-4.

## Test Procedure

1. TC-882's extension with its typed-node schema removed.
2. An extension whose production begins with `function`.
3. Two selected extensions whose productions both begin with `retention`; two whose node kinds are both `retention`.
4. Two extensions whose dependency edges name each other.
5. A unit declaring TC-882's form without selecting the extension.

Tag the tests `#[trace("TC-883", "FR-354-AC-n")]`.

## Expected Results

- Step 1: `invalid_package`/`missing-member` naming the extension and the typed-node schema.
- Step 2: `invalid_package`/`conflicting-definition` naming the extension, the core grammar and `function`.
- Step 3: `invalid_package`/`conflicting-definition` naming both extensions, with the keyword, then with the node kind.
- Step 4: `invalid_package`/`definition-cycle` naming both in path order.
- Step 5: `unsupported_construct`/`declaration-form` naming the form, its span and the unit's selections.
