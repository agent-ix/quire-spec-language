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

### Disposition round 2 — supplemental source check

Frozen head 59144f865da3b61e8b06f73cac182c2e64df889a.

Supplemental source check of commits 852a434b6 and 59144f865: no new finding. The final-carry fixture still invokes the genuine monitor-plan writer, proves nonempty selected-model populations, checks cached indices against the semantics-owned predicate over final wire nodes, preserves original record/source identity and verifies the real Pair rename/rekey. It no longer claims the unrelated monitor-entry ABI passes strict admission. That failure is retained at /semantic_graph/nodes/68/body/members/5 as IllTyped/TypeMismatch in `/tmp/ir1389-carry-refusal-diagnostic-exact.log`; no fabricated admitted package or weakened native success assertion is introduced. The separately admitted combined fixture still executes identity_rows and owner projections with exact and insufficient budgets. The updated existing checked-handoff writer test checks the real final QSL package-copy Rows cause, work/storage code, configured bound, setting and actual over-limit amount, and retains the no-output-on-refusal oracle. Focused fix3 runtime remains queued, so this source supplement claims no runtime pass and does not require IR1397/PLAT1265.

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | still-open | Authentic later-copy allocation controls and writer-boundary resource assertions remain substantive, but the independently expected external row-index oracle still requires the actual owner AdapterRow(i) API. The revised genuine writer-carry test validates metadata against final wire membership and original/rekeyed record values without forging native admission; full native identity value/budget evidence remains in the separately admitted combined fixture. |

### Disposition round 3 — final writer fixture source check

Frozen head c3d1c16fd5ad0e52b2c383dad6d6de422ac4a50f.

Supplemental fixture-only inspection since 59144f865: no new finding. The final writer-carry fixture now supplies two authentic domain documents through their normal owner package intake and evidence APIs. ConfigVersion supplies the real temporal object selection; child_composite supplies genuine nonempty model record fields. Both temporal complete and timed profiles are retained. The oracle still independently enumerates final-node membership through the shared semantics predicates, demands nonempty declaration and field populations, compares cached indices, and checks original source/record identity and fst-to-one rename. Correctly, the named Pair declaration remains stable while its actual record-value content node changes and its old value id is absent. This is a substantive real writer rekey oracle, not a forged admitted package or a claim of strict monitor-root admission. Production implementation is unchanged. The fix8 focused run remains queued, so no runtime result is claimed. Existing AdapterRow and independent row-index findings remain open.

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | still-open | The authentic writer-carry fixture remains sound after correcting its model-field selection and actual named/value rekey expectation. The separate independently expected external row-index oracle still requires the real owner AdapterRow(i) seam; runtime fix8 is pending. |

### Disposition round 4 — focused runtime and caller compatibility

Frozen head 395823b9795238a20eabd8c403b7c384a9d31c47.

Supplemental source and runtime evidence check at the clean pushed head: no new finding. The public writer test calibrates over an explicitly identical package name and first asserts complete byte equality between its measured emission and the actual public writer emission, then still requires exact allowances to write and each one-short resource to refuse with no directory. This corrects an oracle-fixture mismatch without weakening its assertions. The borrowed supply helper explicitly rejects the impossible adapter Rows condition rather than relabelling it. The predecessor snapshot test replaces the removed budget Clone with observation of all six public ledger fields: original work/storage limits, cumulative charge, last success delta, first unpaid charge and first unpaid locus. Its prior-consumption and unchanged-refusal ledger assertions remain intact.

Runtime receipts inspected (reviewer ran no Cargo): `/tmp/ir1389-fix8-focused-root-compatibility-compile-failure.log` contains 9/9 identity tests, 2/2 cold-copy/catalog tests, package Clippy and root library check passing, followed by a preserved root test compilation failure from the removed Clone and new Rows arm. The latter compatibility failures were corrected, and `/tmp/ir1389-root-caller-fix10-focused.log` shows the public writer and predecessor snapshot tests each pass 1/1 on the current source; formatting and diff checks also passed in the coder receipt. This is focused evidence, not a final combined-tree gate. The actual owner AdapterRow API and independent row-index oracle remain open.

Additional examined units:

```yaml
scope:
- id: FR-531-AC-24
  path: spec/functional/FR-531-evaluate-from-a-checked-package-read-from-its-bytes-alone.md
  role: examined
  excerpt: '`V2ReadLimits` has a required `storage` member and no `Default`; a read''s
    evaluation view over limits whose `storage` is exactly the measured join storage
    succeeds and over one unit less returns `ViewFailure::Exhausted` with `ExhaustedStorage`,
    whose refusal names `i2.native_join_storage`; `EvidencePackage::from_read` and
    `from_emission` over a budget one unit short of the source join return `PackageMismatch::Exhausted`;
    the handoff writer over `native_join` one unit short returns `Error::NativeJoinExhausted`
    and writes no directory.'
- id: FR-052-AC-20
  path: spec/functional/FR-052-publish-native-temporal-evaluation-owner.md
  role: examined
  excerpt: Full-predecessor issuance, reading and validation charge their stated native-owner
    work/storage limits before inspection or allocation; exact measured limits admit,
    one less returns the original typed resource/ceiling/attempt with no partial relation,
    and existing caller NativeJoinBudget consumption is never reset.
```

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | still-open | The authentic later-copy, writer-carry and public boundary tests now have focused runtime evidence. The independently expected original external row-index assertion still requires the actual owner AdapterRow(i) seam. |

## Disposition round 5 — combined stack

Candidate `a122d51a2e6d1e4a2f250b801bbddc9cdde1c5d0`. Historical finding text is unchanged.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | The original later-copy and original external AdapterRow index gap is fixed. output_adapter_denials_use_the_original_row_channel_indices derives Tagged/Shape positions from authentic retained slices, runs actual output loops after native joining, observes paid storage prefix and retains the first denial across another complete adapter call. sparse_producer_input_selection_keeps_its_retained_field_index selects an authentic later field index without renumbering and checks both work and storage. Earlier later-copy allocation controls retain zero denied allocations. |

Current original text:

```rust
assert_eq!(budget.first_unpaid_locus(), Some(expected));
```

## Current evidence and limitations

Computed `quire matrix --scope . --format json` produced zero JSON bytes after more than seven minutes of CPU work and was explicitly stopped with SIGTERM. A scoped manual tag/test/source inventory was used; repository-wide untagged counts, matrix completeness and a global matrix PASS are unavailable. Plan completion: not assessed.

Focused reviewer runtime receipts are in this scratchpad. Coder clippy/fmt receipts are secondary; the user assigned the aggregate checks, cargo deny and final gate to the lead, so they were not duplicated. Standalone hybrid_probe.rs uses public APIs linked to the freshly built current qsl_replay library, outside the frozen tree, and reproduces wrong-claim binding without mutating the candidate. No review clone, source edit, commit or push was made.

Focused frozen-head checks: original AdapterRow output, sparse retained field index, tick complete three-resource handoff, root refinement false replay, dependency-isolated refinement false replay, hyper fairness references and two-alias model-root subjects each passed one exact test. Map/sequence actual native projection and cold equality payload tests also passed. The model-enum native closure check FAILED at monitor_root.rs:1308 (expected one Tier row; actual []); historical IR1388 gap FND001 remains still-open. These are focused checks only.
