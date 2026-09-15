---
id: TC-141
title: "Evaluate admitted version-2 compensation expressions"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-049
    type: verifies
  - target: ix://agent-ix/quire-spec-language/NFR-009
    type: verifies
---
# TC-141: Evaluate admitted version-2 compensation expressions

## Description

Verify that the public state evaluator executes the exact guard, retry and
recovery value handles from a strictly admitted compiled-protocol `/2` package
without translating the package or weakening its authority and resource rules.

## Test Procedure

Compile and strictly read the real native compensation fixture as `/2`. Select
each compensation's authored guard, retry and recovery handle from the admitted
package and supply its exact typed binder, population and authority inputs.
Evaluate each handle with sufficient limits and repeat the same call.

Independently cross the declaration, expression handle, compiled artifact,
binding authority, model, anchor and population selections. Evaluate a
compiler-produced `/2` package before it has an independently admitted artifact
identity. Remove one required input. Finally, rerun an exercised dimension with
zero, the exact successful usage and one less than that usage, then retry the
original request with fresh sufficient limits. Exercise the unchanged
version-1 entry point in the same crate build.

## Expected Results

The three exact `/2` compensation expressions return their independently
expected Boolean values and exact replay returns the same value and usage.
Every crossed or unpublished selection returns a typed refusal before runtime
authority is accepted. Missing data returns `Incomplete`; insufficient work
returns `Exhausted` with its exact dimension and no partial value; a fresh retry
completes. Version-1 evaluation remains source-compatible. The implementation
uses one shared evaluator over the admitted package view and performs no JSON
round-trip, public version conversion or second expression interpretation.
