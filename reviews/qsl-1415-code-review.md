---
id: SR-202610310137
title: "Code review — QSL-1415 checked temporal limits v2"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; src/temporal/, src/checking/composed/work.rs, src/protocol_artifact/work.rs, tests/it/checked_temporal_limits_v2.rs, tests/it/compiled_protocol_v2.rs, tests/support/, spec/test-cases/TC-124-bound-temporal-evaluation.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/NFR-008
    type: reviews
---

## Summary

Reviewed PR #2007 at head `20ed8801881d3ca9f6477bda7e22bac5cb864281` against base `e8ac254a6b2f1a7e4258bc364a7eff266ef575e7`, including the Rust implementation, successor tests, and TC-124/NFR-008 AC-1..5. The explicit heap traversal and 100,000-deep small-stack controls are well aligned with the requested successor, but checked-v2 still clamps caller limits and one accounting path charges eviction-list length as retained state.

## Verdict

**CONDITIONAL** — two medium findings remain in the checked-v2 resource contract.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Checked-v2 clamps every caller ceiling to its default, so callers cannot raise limits as NFR-008 requires and above-default configurations collapse to the same effective limit identity. | src/temporal/budget.rs:52 |
| FND-002 | medium | Checked-v2 charges the number of supplied eviction records as peak retained state before determining whether those records are required, so irrelevant eviction entries can spuriously exhaust the retention ceiling. | src/temporal.rs:978 |

## Scope

- NFR-008-AC-1 — checked future/past horizon arithmetic and typed overflow refusal; examined.
- NFR-008-AC-2 — typed stops in all seven dimensions, no truth for the affected obligation, and sibling preservation; examined.
- NFR-008-AC-3 — retention and forced-eviction behavior; examined.
- NFR-008-AC-4 — effective-limit identity and 100,000-deep formula on a 512 KiB stack; examined.
- NFR-008-AC-5 — first unaffordable charge, usage, and retry from unchanged inputs; examined.
- TC-124 checked-package v2 successor description and test bindings; examined.
- `AGENTS.md`, `CLAUDE.md`, `CONTRIBUTING.md`, workspace Cargo lints, and Rust review conventions; context only.

## Detailed Findings

### FND-001 — caller ceilings above defaults are not used

`Limits::bounded` clamps all seven published dimensions to `Limits::default()`. `evaluate_checked_v2` reports and records those clamped limits, so `visits: usize::MAX` becomes the default ceiling. The new identity test explicitly asserts this clamp. NFR-008 says callers may raise or lower each limit and use it as given; AC-5 also requires a retry under sufficient ceilings. A workload above the default can therefore still refuse after the caller raised its ceiling, and the changed caller configuration is indistinguishable from the default in result identity.

### FND-002 — eviction schedule size is charged as retained state

`run_checked` charges `trace.evicted.len()` to the peak `Retention` dimension before resolving the clause or checking whether any listed record is required. The documented unit is the greatest number of retained valuations an unsettled obligation still requires, not the number of eviction instructions. A trace with more irrelevant eviction entries than the retention ceiling is refused as `Retention` even if no required record is evicted and no retention demand reaches that ceiling. The scan may need work accounting, but that does not make its input length retained state.

## Verification

- Manually mapped each NFR-008-AC-1..5 binding in `tests/it/checked_temporal_limits_v2.rs`; each AC has tagged tests and the module is included by `compiled_protocol_v2.rs`.
- Inspected all changed paths and confirmed no legacy test or API deletion in the diff.
- `git diff --check` passed.
- Did not rerun tests or broad build gates per the review dispatch; the coder-reported focused test, no-default, Clippy, and formatting results were treated as supplied evidence.
- `quoin matrix --repo . --json` produced no output and remained running for over 35 seconds; it was interrupted. Manual traceability was used instead.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-003 | medium | Eviction records are now scanned without a charge for each entry, so large eviction lists can multiply atom/capture lookup work beyond the configured dimensions. | src/temporal/formula.rs:665 |
| FND-004 | medium | Retained-demand state is inserted before its retention charge succeeds, performing the first unaffordable retention operation before refusing. | src/temporal/formula.rs:662 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | - |
| FND-002 | fixed | - |
| FND-003 | still-open | The fix removed the eviction-list length charge, but formula and activation paths still scan the complete list without charging scan work; a large list can cause unbounded or multiplied work outside the counters. |
| FND-004 | still-open | `BTreeSet::insert` mutates/allocates the retained-demand entry before `Work::charge` can refuse it. Compute and charge the next distinct demand before inserting. |
| FND-003 | fixed | - |
| FND-004 | fixed | - |
| FND-005 | still-open | The retained instance's `receipts` vector is mutated before the new scan calls `work.visit()?`; visit exhaustion therefore leaves ledger state changed before refusal, and a retry can append the same receipt again. |
| FND-005 | fixed | - |
| FND-006 | still-open | The retained `Instance` is deep-cloned before payload comparison and before any eviction-scan visit is charged. A refusal can therefore perform an unbounded copy of receipts, captures and payload before stopping. |

## Disposition evidence

- FND-001 fixed: `Limits::bounded` now returns caller-supplied limits unchanged.
- FND-002 fixed: checked-v2 no longer charges the total eviction-list length as retention; the new irrelevant-eviction regression test retains only the demanded valuation count.

## New findings (disposition pass 2)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-005 | medium | A work-limit refusal while scanning evicted captures occurs after a receipt is appended to the retained instance, so retrying with sufficient limits can append the same receipt twice. | src/temporal/activation.rs:140 |

### FND-005 — retained receipts mutate before eviction-scan exhaustion

The retained-instance path appends `trigger.receipt` to the ledger's saved instance before scanning the eviction list with `work.visit()?`. If the visit ceiling is exhausted during that scan, evaluation returns an error after mutating caller-owned `Ledger` state. Retrying the same trigger after raising the ceiling appends the same receipt again, so the retry is not from an unchanged state.

### Disposition pass 2 evidence

- FND-003 fixed: formula, activation, and protocol preflight scans now charge visit/work accounting before inspecting eviction candidates; focused tests cover formula and capture scan exhaustion.
- FND-004 fixed: checked-v2 computes prospective distinct valuation demand, charges it, and only then inserts the record; unit coverage verifies a refused charge leaves the set empty and duplicate demand charges once.


### Disposition pass 3 evidence

The retained-instance branch now clones the saved instance, completes and charges the eviction scan, and only then appends the receipt to the cloned assessment and the ledger. The new AC-5 regression test seeds a ledger, forces a visit refusal during that scan, retries the same delivery with a sufficient ceiling, and asserts the receipt appears once while the original capture value is preserved.


## New findings (disposition pass 3)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-006 | medium | The retained instance is deep-cloned before payload validation and before the bounded eviction scan, so a contradictory input or visit refusal can trigger an uncharged copy of accumulated receipts, captures and payload. | src/temporal/activation.rs:129 |

### FND-006 — retained instance is copied before the refusal boundary

The new `.cloned()` at the retained-instance lookup deep-copies `Instance` before checking the payload and before the loop charges each eviction candidate. `Instance` contains a `Vec<String>` of receipts, captures and an opaque payload; receipts can accumulate across repeated deliveries. A contradictory redelivery or a low visit limit therefore performs the deep copy even though evaluation immediately refuses. The previous implementation borrowed the instance through payload validation and the scan, cloning only after a successful scan. Borrow through comparison and scan, then clone or update after the work charges succeed.
