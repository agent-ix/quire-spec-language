---
id: SR-202611070458157
title: IR-1389 gap-analysis review
type: SpecReview
analysis: gap-analysis
scope: agent-ix/quire-spec-language; IR-1389; PR2500; identity materialization and
  native_row helpers; FR-531 AC31/33; TC-1331
review_set: subset
---

## Summary

Manual criterion-to-test review finds authentic value, exact-budget, composition and initial zero-allocation controls, but lacks a later real-row denial oracle.

Frozen head: 1efa1e56c582cf137d456e8669c750d46aea43e5. Ticket: IR-1389.

## Verdict

**CONDITIONAL — later authentic record/member and union allocation-prefix controls are now present, but the independently expected real row-index assertion remains impossible until the actual owner AdapterRow(i) seam is consumed.**

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Denial controls exercise initial input/output storage and whole-call one-short limits, but never a later real record/member/union copy with an independently expected row locus and observed admitted prefix; add that adverse-case oracle so wrong row indices or a denied payload allocation can fail the test. | qsl-package/src/emit/tests/identity_budget.rs:146-181,247-273 |

## Coverage

Examined the full scoped diff, both production boundary callers, identity input/output preparation, final writer carry, compact first-denial metadata and actual owned copy paths. Shared generic helpers are being moved from the same repository's unmerged PR2477 into this owning PR, not vendored across repositories. No applicable AssuranceProfile was found under spec/. No CI lanes or warning thresholds changed.

Tests directly compare authentic declaration/field populations, two source records, one union, member counts/order/types and owner projections. Phase accounting, exact allowances, repeated-call consumption, zero initial storage, signed cold helper values and authentic writer carry are covered by runnable bodies. The missing later-row adverse control is separate from the correct first-denial-to-budget equality assertions. Computed matrix attempt with --prefix/--json was rejected by this installed CLI; the supported --format json invocation timed out after 45 seconds (exit 124, empty output), so no computed matrix pass is claimed. Manual bindings are disclosed here. No hand-written matrix was modified. Plan completion: not assessed.

Reviewer ran no Cargo or aggregate gate. At source-review time the current focused run remained queued; earlier five core tests passed, and the invalid authored >i128 fixture was corrected to the actual admitted i128 domain while generic owner cold/signed controls remain separate. Final aggregate verification belongs to the lead. Quire validates both changed spec documents with no structural errors; ambient duplicate-archetype warnings are environmental. Optional Jev criterion-strength and unrelated object/dependency/security analyses were not selected.

## Examined units

```yaml
scope:
- id: FR-531-AC-31
  path: spec/functional/FR-531-evaluate-from-a-checked-package-read-from-its-bytes-alone.md
  role: examined
  excerpt: An authentic emission with source records, record members and unions produces
    complete identity rows equal to the original producer claims after native joining
    and QSL materialization. The measured work and storage include the QSL producer
    input copies, the single native join and each later QSL declaration, field, record/member
    and union copy. Exact allowances admit; one unit below either measured allowance
    refuses at its first unpaid step with no partial identity rows, the original configured
    bound and the admitted prefix intact. Two materializations on one budget compose
    without re
- id: FR-531-AC-33
  path: spec/functional/FR-531-evaluate-from-a-checked-package-read-from-its-bytes-alone.md
  role: examined
  excerpt: Allocation observation encloses only the adapter/conversion call, with
    fixture preparation and assertions outside it. A successful owned materialization
    performs observed allocations; a denial before its first allocation performs none,
    and a later denial allocates neither the denied material nor an owned diagnostic.
    The controls include fresh owner-produced lazy numeric payloads, values `2^127`
    and `-(2^127 + 1)`, and larger heap-backed values without prior warming or formatting.
    These values remain exact on successful copies; insufficient work/storage retains
    the first real row locu
- id: FR-531
  path: spec/functional/FR-531-evaluate-from-a-checked-package-read-from-its-bytes-alone.md
  role: examined
  excerpt: 'The `identity_rows` adapter SHALL charge its QSL-owned materialization
    in

    addition to the native identity join. It SHALL charge owned producer input

    rows before presenting them to the native join, including copies an input

    iterator would otherwise create while the join first peeks at a row.'
- id: TC-1331
  path: spec/test-cases/TC-1331-evaluate-from-a-checked-package-read-from-its-bytes-alone.md
  role: examined
  excerpt: 'Measure producer input copies and the single native join separately from
    the adapter''s later declaration,

    field, record/member and union copies, requiring their sum to equal the shared

    budget''s cumulative consumption.'
```

## Dispositions

Round 1, frozen head 6df9d8b9592adeb798cb85e1d6afb432c3426400.

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | still-open | New production copy_source_record/copy_source_union controls observe the paid Point prefix separately and demand zero allocations for denied Tagged member and Shape payload copies with value/resource/phase assertions. They still pass Package loci and do not assert an independently expected original row index; runtime is queued, so the complete finding is not fixed. |

Source inspection only. Focused runtime remains queued in `/tmp/ir1389-fix1-focused.log`; no reviewer Cargo or aggregate gate was run and no runtime pass is claimed. New helper extraction keeps the actual production copy paths under observation; it does not invent authoritative rows.
