---
id: SR-202611070458156
title: IR-1389 code-review review
type: SpecReview
analysis: code-review
scope: agent-ix/quire-spec-language; IR-1389; PR2500; identity materialization and
  native_row helpers; FR-531 AC31/33; TC-1331
review_set: subset
---

## Summary

Rust implementation, API and code-test review identifies the known missing row-locus seam and a duplicated live selection rule.

Frozen head: 1efa1e56c582cf137d456e8669c750d46aea43e5. Ticket: IR-1389.

## Verdict

**FAIL — FND-002 is fixed by independent source inspection; FND-001 remains open pending the actual owner row-locus API.**

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | External field, record, member and union copy denials use only Package plus a channel form, losing the original row index; consume the real owner AdapterRow(i) seam before merge and preserve per-channel retained indices. | qsl-package/src/emit/identity_rows.rs:107-114,138-161,184-190 |
| FND-002 | high | The new membership cache duplicates the live node/form classification already owned by qsl_semantics::model::intake::DeclarationNodeKind and selected_exact_identities_from_rows; share one classification/predicate while retaining the precomputed final-emission membership. | qsl-package/src/emit/identity_rows.rs:32-58 |

## Coverage

Examined the full scoped diff, both production boundary callers, identity input/output preparation, final writer carry, compact first-denial metadata and actual owned copy paths. Shared generic helpers are being moved from the same repository's unmerged PR2477 into this owning PR, not vendored across repositories. No applicable AssuranceProfile was found under spec/. No CI lanes or warning thresholds changed.

Duplication search: `DeclarationNodeKind` and `selected_exact_identities_from_rows` in qsl-semantics/src/model/intake.rs:79-156 remain used by qsl-semantics/src/model/intake/unit.rs:126 and qsl-semantics/src/check/mod.rs:7863. New identity_rows.rs:32-58 repeats those four declaration-form/tag pairs and the record field predicate. A shared borrowed classifier/predicate can populate the final-emission cache without allocating in the budgeted adapter.

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
| FND-001 | still-open | Current owner API still lacks AdapterRow(i); production field/record/union/member charges retain Package plus static form, so the original external row index is absent. No completion or mergeability claim. |
| FND-002 | fixed | The semantics-owned shared predicates are used by both the cached selection and original live helper; the duplicate local classification was removed. |

Source inspection only. Focused runtime remains queued in `/tmp/ir1389-fix1-focused.log`; no reviewer Cargo or aggregate gate was run and no runtime pass is claimed. New helper extraction keeps the actual production copy paths under observation; it does not invent authoritative rows.

### Disposition round 2 — supplemental source check

Frozen head 59144f865da3b61e8b06f73cac182c2e64df889a.

Supplemental source check of commits 852a434b6 and 59144f865: no new finding. The final-carry fixture still invokes the genuine monitor-plan writer, proves nonempty selected-model populations, checks cached indices against the semantics-owned predicate over final wire nodes, preserves original record/source identity and verifies the real Pair rename/rekey. It no longer claims the unrelated monitor-entry ABI passes strict admission. That failure is retained at /semantic_graph/nodes/68/body/members/5 as IllTyped/TypeMismatch in `/tmp/ir1389-carry-refusal-diagnostic-exact.log`; no fabricated admitted package or weakened native success assertion is introduced. The separately admitted combined fixture still executes identity_rows and owner projections with exact and insufficient budgets. The updated existing checked-handoff writer test checks the real final QSL package-copy Rows cause, work/storage code, configured bound, setting and actual over-limit amount, and retains the no-output-on-refusal oracle. Focused fix3 runtime remains queued, so this source supplement claims no runtime pass and does not require IR1397/PLAT1265.

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | still-open | The owner AdapterRow(i) variant is still absent, so external row denials still use Package/form. The new root-writer one-short assertion correctly expects inline Rows and checks actual resource, bound and final package-copy locus; it does not resolve missing external row indices. |
