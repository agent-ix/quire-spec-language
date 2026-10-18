---
id: TC-1074
title: "Keep an unordered capture graph out of protocol slot evaluation"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-390
    type: verifies
---
# TC-1074: Keep an unordered capture graph out of protocol slot evaluation

## Description

Verify that a capture graph with a foreign or recursive initializer never
reaches the evaluator, because S3 refuses it from source and the package reader
refuses it from bytes, and that protocol slots share the one `ProtocolClause`
arm of the S6a seam. Scope: FR-390-AC-13 and FR-390-AC-14.

Catches a package whose capture reads the capture itself, a later capture or
another template's capture and still evaluates, a reader that accepts an
initializer cycle, and a second S6a arm added for protocol slots.

Fixture **E** and the observation of TC-1070.

## Test Procedure

| # | Run | Expected |
| --- | --- | --- |
| 1 | check the unit with `capture target: M::Total = target;` in `Refund` | S3 refuses `missing_declaration`/`missing-name` at the read with `read` `forward` (TC-1057 row 8); no checked package is built, so no slot can be evaluated |
| 2 | check the unit with the registration captures `capture a: Boolean = b;` then `capture b: Boolean = true;` | S3 refuses with `read` `forward` (TC-1057 row 9) |
| 3 | check the unit whose `Partial` registration capture reads `target` of `Refund` | S3 refuses with `read` `foreign` (TC-1057 row 11) |
| 4 | compile the unit, then in the emitted bytes retarget the initializer of `activated` to the node of `Refund`'s retry relation, which reads `activated` | the reader refuses the bytes `invalid_package`/`definition-cycle`, naming the nodes on the cycle in tree order; the I2 read-back returns no package |
| 5 | compile the unit, then in the emitted bytes retarget the initializer of `activated` to the binder node of `partialTarget` | the reader refuses the bytes `missing_declaration`/`missing-name` at the read, `read` `foreign`, stage `activation-capture`, owner `Refund` |
| 6 | compile the unit and evaluate `Refund.retry.relation` with TC-1070 row 8's supply | evaluates: true, `captures()` `target`, `activated` (the control: the unmutated package reads back and evaluates) |
| 7 | add the S6a probe variant of FR-107-AC-6 | the build fails with E0004 at exactly the checked-in S6a seam list; its `evaluate_declaration` match has the one `ProtocolClause` arm, and protocol slots add no arm |

## Expected Results

Every capture initializer in a package reads only the sources and the captures
written before it in its own template, so the evaluator computes a capture
from nothing but the supplied values and earlier captures. A graph that does
not satisfy this is refused where it is written (S3) or where it is read (the
package reader) and no evaluation call can be made over it. Protocol slots and
state clauses share the one arm.
