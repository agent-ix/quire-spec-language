---
id: SR-202611070458161
title: IR-1389 failure-domain review
type: SpecReview
analysis: failure-domain
scope: agent-ix/quire-spec-language; IR-1389; PR2500; identity materialization and
  native_row helpers; FR-531 AC31/33; TC-1331
review_set: subset
---

## Summary

The requirement addresses prior denial, zero storage, cumulative consumption, final carry/rekey and allocation-free failure; the contract exposes rather than hides the row-locus owner dependency.

Frozen head: 1efa1e56c582cf137d456e8669c750d46aea43e5. Ticket: IR-1389.

## Verdict

**PASS** — No specification defect found in the examined changed scope; implementation readiness is recorded separately.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Coverage

Examined the full scoped diff, both production boundary callers, identity input/output preparation, final writer carry, compact first-denial metadata and actual owned copy paths. Shared generic helpers are being moved from the same repository's unmerged PR2477 into this owning PR, not vendored across repositories. No applicable AssuranceProfile was found under spec/. No CI lanes or warning thresholds changed.

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
