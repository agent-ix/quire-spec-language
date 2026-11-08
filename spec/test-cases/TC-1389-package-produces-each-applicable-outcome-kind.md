---
id: TC-1389
title: "package produces each applicable outcome kind of the lifecycle matrix"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-564
    type: verifies
---
# TC-1389: package produces each applicable outcome kind of the lifecycle matrix

## Description

Run the `package` row of FR-564's matrix through the library.

Scope: FR-564-AC-2.

## Test Procedure

Run `package` over each applicable cell's input: FR-275-AC-1's checked
package (success); FR-185-AC-7's `Service` (unsupported); the FR-275-AC-1
package with a handle cancelled `Deadline` and then `Requested` before the
call; and with a `package` limit field one below the counter that input
reaches (FR-277-AC-1).

Tag the tests `#[trace("TC-1389", "FR-564-AC-2")]`.

## Expected Results

Success exit 0; `unsupported_construct` exit 21; `Cancelled(Deadline)`,
`Cancelled(Requested)` and `LimitExceeded` naming the field, each exit 22.
