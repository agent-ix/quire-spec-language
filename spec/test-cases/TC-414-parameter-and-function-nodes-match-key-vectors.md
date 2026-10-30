---
id: TC-414
title: "Parameter, literal and function nodes retain their checked shapes"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-092
    type: verifies
---
# TC-414: Parameter, literal and function nodes retain their checked shapes

## Description

Verify FR-092-AC-4/5/6/10: parameter level and type, function parameter
order, separate expression nodes, literal values and measure references.
Compare node ids as QSpec FR-322 structural `{package, path, ordinal}`
objects, never as preimage bytes or digests.

## Test Procedure

1. Check the `both`, `f`, `h` and `k` functions of FR-092-AC-4. Inspect
   parameter levels and types, literal values, function bodies and node ids.
2. Check `unused` and `both2` of FR-092-AC-5. Inspect the full ordered
   parameter list including the unread parameter.
3. Walk each function body for an inline application; inspect the separate
   `a and b` application node and its structural id.
4. Check `m` with `decreases(x)` and inspect its measure reference.

## Expected Results

The node shapes match FR-092. Function bodies contain references but no
inline application, the unread parameter remains, the integer literal is
exact, and `decreases` references its parameter. Every node id satisfies
FR-322's structural identity rule and equals another only when all three
components equal. No T/D/G/L/E/P vector or preimage byte is an oracle.
