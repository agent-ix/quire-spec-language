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
| FND-006 | fixed | - |
| FND-007 | still-open | A retained receipt is appended and the instance cloned after the eviction scan but before formula evaluation; if the next formula visit exhausts the limit, the ledger is mutated and retry appends the receipt again. |
| FND-001 | fixed | - |
| FND-002 | fixed | - |
| FND-003 | fixed | - |
| FND-004 | fixed | - |
| FND-005 | fixed | - |
| FND-006 | fixed | - |
| FND-007 | fixed | - |
| FND-008 | still-open | Full retained instance cloning remains before the formula root's visit charge; defer it until formula work has returned. |
| FND-001 | fixed | - |
| FND-002 | fixed | - |
| FND-003 | fixed | - |
| FND-004 | fixed | - |
| FND-005 | fixed | - |
| FND-006 | fixed | - |
| FND-007 | fixed | - |
| FND-008 | fixed | Retained-instance and new retained-ledger instance clone costs are charged before cloning. |
| FND-009 | still-open | Clone byte estimates are charged to `Visits`, whose published unit is temporal graph-node visits; this changes usage and can refuse an evaluation before the node-visit ceiling is reached. |
| FND-001 | fixed | - |
| FND-002 | fixed | - |
| FND-003 | fixed | - |
| FND-004 | fixed | - |
| FND-005 | fixed | - |
| FND-006 | fixed | - |
| FND-007 | fixed | - |
| FND-008 | fixed | - |
| FND-009 | fixed | Retained-instance clones and ledger receipt commits are deferred until formula evaluation returns; Visits remains graph-node visits. |
| FND-010 | still-open | A settled sibling's receipt is committed before a later sibling can exhaust; retrying the same trace stages and commits that already recorded receipt again. |

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


## New findings (disposition pass 4)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-007 | medium | A visit-limit refusal at the formula root can occur after the retained receipt is appended, so the retry still duplicates that receipt when the limit is consumed by activation and eviction scanning. | src/temporal/activation.rs:164 |

### FND-007 — later work refusal still mutates retained receipts

The new fix postpones receipt mutation until the eviction-list scan completes, which resolves FND-005's scan-refusal case. However, `saved.receipts.push(...)` and `saved.clone()` still run before `run_view` evaluates the formula. If the visit limit covers trigger processing and the entire eviction scan but not the formula root visit (`Evaluator::evaluate` calls `work.visit()` on entry), evaluation then refuses after mutating `Ledger.checked_authenticated_instances`. Retrying the same input with sufficient visits appends the same receipt again. Add a checked-v2 retry case where the scan completes but the first formula visit exhausts, and defer the ledger mutation until the evaluation can commit it.

### Disposition pass 4 evidence

- FND-001 through FND-004 remain fixed: caller-supplied ceilings pass through, retention tracks distinct required valuation demand, eviction scans charge `Visits`, and new demand is charged before insertion.
- FND-005 remains fixed for the reported scan-refusal case: the retained-instance branch completes the charged eviction scan before appending a receipt.
- FND-006 is fixed: the retained `Instance` is borrowed for payload validation and eviction scanning; the clone occurs only after the scan completes.
- FND-007 remains open: successful activation mutates the retained receipt before the formula root's first visit, which can still exhaust the shared visit ceiling.

## New findings (disposition pass 5)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-008 | medium | The retained instance, including accumulated receipts and captures, is still deep-cloned after the eviction scan but before formula work is charged; an immediate formula visit refusal therefore performs an uncharged copy of caller-retained state. | src/temporal/activation.rs:158 |

### FND-008 — retained instance copy precedes formula work refusal

The retained-state branch now validates the payload and charges the complete eviction scan before cloning, which closes FND-006 as written. It still clones the full `Instance` and appends the receipt at `activation.rs:158-163`, before `run_view` invokes the formula evaluator. With a visit ceiling that pays for trigger handling and scanning but not the formula root, `work.visit()` refuses after the deep copy. Because `Instance` carries potentially accumulated receipts, captures and payload, this remains unbounded work before the next charged operation. Keep the retained state borrowed through formula evaluation, then materialize the assessment and commit receipt updates only after its outcome is available.

## Disposition pass 5 evidence

- FND-001 through FND-006 remain fixed on head `3d0015d388665d1516bebcadebbca94b17777e88`; caller limits pass through, retention charges demanded state, eviction scans charge visits, new retained demands are charged before insertion, scan refusal does not commit receipts, and payload validation plus scan precede retained-state cloning.
- FND-007 is fixed: `run_view` now returns immediately on `Error::Exhausted` before ledger receipt commit; the new `checked_v2_formula_refusal_retries_without_duplicate_receipt` regression forces formula-root visit exhaustion after a successful scan and confirms the retry reports one receipt and the retained capture.
- FND-008 remains open: the successful-scan clone at `src/temporal/activation.rs:158-163` still copies the retained `Instance` before the formula root's next visit can refuse. This is distinct from FND-007's corrected receipt mutation: the ledger remains unchanged, but potentially large state is still copied before an unaffordable visit.


## New findings (disposition pass 6)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-009 | medium | Clone byte estimates are charged to the `Visits` counter, although NFR-008 and the public `Limits`/`Usage` API define visits as temporal graph-node visits; result usage and visit ceilings now mix incompatible units. | src/temporal/activation.rs:161 |

### FND-009 — copy-byte charge changes the published Visits unit

The helper `instance_clone_work` returns an estimated byte count, then callers add that value to `Charge::Visits`. The NFR-008 counter contract defines `visits` as temporal graph-node visits, and the public `Limits` and `Usage` docs repeat that unit. This change can spend most or all of a caller's visit ceiling copying a receipt-heavy instance before evaluating any graph node, and reported `Usage.visits` no longer means graph visits. Either keep cloning after formula work has succeeded, or define and version a counter model whose units accurately include this work without conflating it with graph visits.

## Disposition pass 6 evidence

- FND-001 through FND-007 remain fixed on head `7f3862a821192d2dc464f6a7414dcd5421c52e46`: caller ceilings, retention demand, scan accounting, charge-before-insert, retry-safe receipts, pre-scan validation, and post-formula receipt commit remain in place.
- FND-008 is fixed: `instance_clone_work` is charged before cloning both restored instances and instances copied into the retained ledger. The updated formula-refusal regression probes successful visit usage, sets a one-short ceiling, confirms refusal is at a formula node, and retries with the default ceiling while asserting one receipt and preserved capture.
- FND-009 remains open: the precharge reuses a counter whose published unit is graph-node visits, so cost accounting changes API semantics and result usage.


## New findings (disposition pass 7)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-010 | medium | When one trigger instance settles and is committed to the ledger before a later sibling exhausts the shared work limit, retrying the same trace appends the first instance's already committed receipt again. | src/temporal.rs:954 |

### FND-010 — settled sibling receipt duplicates on whole-trace retry

`run_view` commits each successful instance's state and receipts inside the loop. If the next instance returns `Error::Exhausted`, the function returns with the earlier sibling still committed. On retrying the same trace, activation resolves that first trigger from the retained ledger and stages its receipt again; the successful retry extends that receipt a second time. The existing sibling-exhaustion test checks the returned settled assessment but uses no mutable `Ledger`, so it does not detect the repeated receipt. Ensure the retry path recognizes an already committed delivery receipt or otherwise makes the multi-instance commit/retry idempotent.

## Disposition pass 7 evidence

- FND-001 through FND-008 remain fixed on head `5f5d9f5e81ca65419fab782be81a09fe98a6195e`: the prior limit, scan, retention, receipt, and pre-refusal-copy fixes remain present.
- FND-009 is fixed: `Resolved::Retained` keeps only identity and new receipts through formula work; saved receipts and the retained-instance clone occur after an outcome, while new instances enter the ledger only after their formula settles. The updated regression asserts graph visit usage is exactly three and retries without duplicate receipts for one instance.
- FND-010 remains open: per-instance commits are not atomic or deduplicated across a later sibling's visit exhaustion, so the same trace can append an already committed sibling receipt during retry.
