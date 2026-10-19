---
id: TC-1080
title: "A refused automaton state key rejects the closure certificate and the proof does not settle proved"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-338
    type: verifies
---
# TC-1080: A refused automaton state key rejects the closure certificate and the proof does not settle proved

## Description

Verify that `check_closure` rejects a certificate holding a key the translation refuses to admit, and that no proof results.

Scope: FR-338-AC-8.

## Test Procedure

Use FR-338-AC-1's certificate over ADR-018 §6's subject.

1. Replace one product state's key with a key of a foreign context.
2. Repeat with a non-canonical encoding and with an unsorted set.
3. Settle each proof through FR-127.
4. Restore the valid key and check again.

Tag the tests `#[trace("TC-1080", "FR-338-AC-8")]`.

## Expected Results

- Steps 1 and 2: `Rejected` with `ShapeMismatch` at `ProductState` carrying the refused key and its model state; never `Accepted`.
- Step 3: `inconclusive`, `CertificateRejected`; never `proved`.
- Step 4: `Accepted`.
