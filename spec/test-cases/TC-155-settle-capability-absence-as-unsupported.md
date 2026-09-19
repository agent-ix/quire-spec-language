---
id: TC-155
title: "Keep admission backend-independent and settle backend absence as unsupported"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-057
    type: verifies
---
# TC-155: Keep admission backend-independent and settle backend absence as unsupported

## Description

Verify that the QSL composed linker admits requested pairs without reading
backend state, and that a requested kind no registered backend advertises
settles `unsupported` with a warning naming it. Scope: FR-057-AC-5 and
FR-057-AC-6.

## Test Procedure

1. Link one protocol declaration with two required requested pairs: one for
   `global-conformance` and one for `realizability`.
2. Admit the pairs three times: with no registered backend, with one backend
   advertising only `global-conformance`, and with two backends advertising
   different kinds. Compare the admitted pairs and static components.
3. With the one-backend registry, run negotiation over the admitted pairs.

## Expected Results

- Step 2: the admitted pairs and static components are identical in all three
  runs.
- Step 3: the `realizability` item settles `unsupported`, with a warning whose
  structured field holds `realizability`. No refusal is produced, and no item
  waits for a registration. The `global-conformance` item settles on its own.
- Because a required item is not `supported`, complete aggregate success is
  unavailable, and no artifact is emitted for the `unsupported` item.
