---
id: TC-884
title: "An extension changes no other unit's identity, backends change no admission, and a reader without it refuses"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-354
    type: verifies
---
# TC-884: An extension changes no other unit's identity, backends change no admission, and a reader without it refuses

## Description

Verify the independence of admission from unselected extensions and registered backends, and a reader's refusal of an unknown extension node.

Scope: FR-354-AC-5.

## Test Procedure

1. Compile a unit that selects no extension over a catalog with and without TC-882's extension.
2. Compile TC-882 step 1's unit with two registered backends, then with none.
3. Read TC-882 step 1's package with a QSL reader whose catalog lacks the extension.

Tag the tests `#[trace("TC-884", "FR-354-AC-5")]`.

## Expected Results

- Step 1: equal checked package identities.
- Step 2: equal checked package identities.
- Step 3: `unknown_required_feature`/`unknown-feature` naming the extension and the `retention` node.
