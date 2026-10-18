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

Verify that a capture that reads itself, a later capture or another template's
capture never reaches the evaluator, because S3 refuses it from source and the
contract reader refuses it from bytes, and that protocol slots share the one
`ProtocolClause` arm of the S6a seam. Scope: FR-390-AC-12 and FR-390-AC-13.

Catches a package whose capture reads itself, a later capture or another
template's capture and still evaluates, and a second S6a arm added for protocol
slots.

Fixture **E** and the observation of TC-1070. Rows 4 and 5 mutate the emitted
bytes as one edit with its consequences. The edit retargets the initializer of
the `activated` capture to a Boolean `expression` node already in the package.
That node is referenced by its parent expression, not by the compensation, so
the edit also adds the node to the compensation's `dependencies`, and it
re-mints the compensation's node key, the clause's node key and every key above
them, and the `package_id`, which the reader recomputes. With those rewritten
the only defect left is the binder-scope read, so the reader reaches its slot
checks (QSpec FR-464 reader order, step 6) and refuses there.

## Test Procedure

| # | Run | Expected |
| --- | --- | --- |
| 1 | check the unit with `capture target: M::Total = target;` in `Refund` | S3 refuses `missing_declaration`/`missing-name` at the read with `read` `forward` (TC-1057 row 8); no checked package is built, so no slot can be evaluated |
| 2 | check the unit with the registration captures `capture a: Boolean = b;` then `capture b: Boolean = true;` | S3 refuses with `read` `forward` (TC-1057 row 9) |
| 3 | check the unit whose `Partial` registration capture reads `target` of `Refund` | S3 refuses with `read` `foreign` (TC-1057 row 11) |
| 4 | compile the unit, then in the bytes FR-383 emits retarget the initializer of `activated` to the node of the subterm `not activated` of `Refund`'s retry relation, which reads `activated`, with the dependencies and keys rewritten as above | the contract reader refuses the bytes `missing_declaration`/`missing-name` at the read, `read` `forward`, stage `activation-capture`, owner `Refund`; it returns no package |
| 5 | compile the unit, then in the bytes retarget the initializer of `activated` to the node of the subterm `partialTarget >= 0` of `Partial`'s trigger guard, with the dependencies and keys rewritten as above | the contract reader refuses `missing_declaration`/`missing-name` at the read, `read` `foreign`, stage `activation-capture`, owner `Refund` |
| 6 | compile the unit and evaluate `Refund.retry.relation` with TC-1070 row 8's supply | evaluates: true (the control: the unmutated package reads back and evaluates) |
| 7 | add the S6a probe variant of FR-107-AC-6 | the build fails with E0004 at exactly the checked-in S6a seam list; its `evaluate_declaration` match has the one `ProtocolClause` arm, and protocol slots add no arm |

## Expected Results

Every capture initializer in a package reads only the sources and the captures
written before it in its own template, so a capture slot computes its value
from nothing but the supplied values. A graph that does not satisfy this is
refused where it is written (S3) or where it is read (the contract reader) and
no evaluation call can be made over it. Protocol slots and state clauses share
the one arm.
