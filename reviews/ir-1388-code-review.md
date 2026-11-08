---
id: SR-20261106200931
title: "IR-1388 PR #2477 code-review"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; IR-1388; qsl-analyze/tests/it/monitor_root.rs; qsl-eval/tests/it/protocol_slots/acceptance.rs; qsl-eval/tests/it/protocol_slots/acceptance/bare_slots.rs; qsl-package/Cargo.toml; qsl-package/src/checked_v2.rs; qsl-package/src/emit.rs; qsl-package/src/emit/equality_reader.rs; qsl-package/src/emit/monitor_plan.rs; qsl-package/src/emit/selected_rows.rs; qsl-package/src/emit/tests.rs; qsl-package/src/emit/tests/equality_budget.rs; qsl-package/src/emit/tests/native_row_budget.rs; qsl-package/src/emit/tests/source_tuple_rows.rs; qsl-package/src/lib.rs; qsl-package/src/native_join_failure.rs; qsl-package/src/native_row.rs; qsl-package/src/reader_protocol_projection.rs; spec/functional/FR-531-evaluate-from-a-checked-package-read-from-its-bytes-alone.md; spec/test-cases/TC-1331-evaluate-from-a-checked-package-read-from-its-bytes-alone.md; src/checked_handoff.rs"
review_set: subset
---

## Summary

Ticket: IR-1388. Rust review inspected all new charge helpers, allocation sites, fixed-size coded refusals, caller propagation, native owner payload clone behavior, private eligibility construction/carry/rekey, and exhaustive equality fact conversion. Ordinary emitted eligibility is sound: its generator first rejects foreign keys and requires an emitted wire node; monitor finalization refreshes after rekey. No unsafe code or workflow narrowing was introduced. The fixed-size error envelope is an intentional user-required zero-storage exception to allocating message/context defaults. Two high integer-accounting defects remain.

## Examined units

- FR-531-AC-29 (examined, spec/functional/FR-531-evaluate-from-a-checked-package-read-from-its-bytes-alone.md): "Materializing an authentic emission's equality witness with a shared native budget preserves every application, fact and source-unit row, including original identities and order. The source admission and handoff paths use the same budgeted row builder. Exact measured work and storage allowances return the complete witness; one unit less in either resource refuses without allocating the denied material or a diagnostic, retains the first unpaid resource, amount and original locus, and returns no partial witness. Two individually affordable materializations that exceed one shared allowance refuse"
- FR-531-AC-30 (examined, spec/functional/FR-531-evaluate-from-a-checked-package-read-from-its-bytes-alone.md): "Source-enum materialization and QSL-to-IR model-enum-shape and selected-collection conversion preserve original structural/declaration/member identities, canonical case order and complete Exact types on one shared native budget. Exact measured bounds admit; work or storage one short and zero storage return allocation-free compact denials at the first unpaid locus with prior consumption intact. A later conversion cannot reset a preceding adapter's charges or replace its first denial. A selected source map whose key has no representation in the target IR row returns a typed unsupported result in"
- FR-531-AC-28 (examined, spec/functional/FR-531-evaluate-from-a-checked-package-read-from-its-bytes-alone.md): "Producer and bytes-only equality admission pass the caller's same `NativeJoinBudget` through IR's admitted equality reader after identity/type projection. Decisions and non-resource witness refusals are retained for later unmetered selection and projection. Successful reads consume native work and storage cumulatively; exact measured bounds admit, and one unit less in either resource refuses with the original typed IR equality denial and recorded first unpaid charge, never an unknown witness or internal fault. Reusing an admitted slot consumes no additional native allowance."
- FR-531 (examined, spec/functional/FR-531-evaluate-from-a-checked-package-read-from-its-bytes-alone.md): "Each adapter SHALL charge its inspections and every owned allocation through\nContract IR's public accounted-charge API before the corresponding inspection\nor allocation."
- TC-1331 (examined, spec/test-cases/TC-1331-evaluate-from-a-checked-package-read-from-its-bytes-alone.md): "Use one genuine emission retaining both a sequence and a map: converting and\nprojecting the selected sequence succeeds with its original type and nodes,\nwhile selecting the map returns its typed unsupported cause and keeps its key."

## Verdict

**FAIL** — Static findings require correction; runtime acceptance remains pending on the owner dependency.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Integer precharge treats every value with 128 magnitude bits as inline, but Exact stores values outside signed i128 on the heap. Positive 2^127 and negative -2^127-1 both have 128 magnitude bits and cloning them allocates a BigInt/limb buffer with no storage charge. Classify actual owned integer storage without this magnitude-only threshold and add both signed boundary controls. | qsl-package/src/native_row.rs:322-327 |
| FND-002 | high | Integer footprint inspection itself can allocate before its storage charge: for a cold LargeDigits value returned by Integer::try_clone, magnitude_bits calls as_big, which initializes a boxed BigInt and clones limbs. An ensuing denied charge has already allocated unaccounted input-cache storage. Use an allocation-free owner inspection/precharge seam and test a cold fallibly copied large integer; do not warm the input through an ample call first. | qsl-package/src/native_row.rs:322-327 |

## Evidence and limits

Reviewed frozen d6f62bc24de15d5ff57406b4dbb391ed7b4a02ac in the existing ticket worktree. IR-1387 PR #943 is not merged; its actual published resume/accounted_charge API was inspected, and the pending lock is a disclosed dependency, not a new code defect. The allocation-counter dev dependency awaits normal lock resolution. No Cargo build, test, Clippy or aggregate gate was run by this reviewer, and no runtime PASS is claimed. Quire validated the two changed spec documents (2/2 grammar-clean). Computed matrix was unavailable after a bounded 45-second timeout (exit 124, zero JSON output); no retry was attempted. Manual trace inspection is disclosed. No AssuranceProfile is declared by the changed artifacts or selected in the dispatch. Plan completion: not assessed.

Owner implementation evidence: locked Exact integer.rs:186-190 normalizes only values fitting signed i128 into inline storage; :348-359 clones heap-backed payloads; :613-616 calls as_big for non-inline values; :1122-1136 allocates the cold LargeDigits cache. These static owner facts establish both reported failures without a test run.

## Dispositions — combined stack round 1

Candidate a122d51a2e6d1e4a2f250b801bbddc9cdde1c5d0. Historical finding rows are immutable.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Integer uses owner copy_footprint, rather than the old 128-magnitude threshold. signed_integer_boundaries_charge_the_owned_storage_their_copy_actually_allocates checks i128 endpoints, both 128-bit out-of-range values and larger signed values against observed actual copy bytes. |
| FND-002 | fixed | Footprint inspection now uses the owner allocation-free seam. cold_fallibly_copied_integer_refuses_storage_without_warming_or_allocating creates fresh try_clone digits and observes no allocation across integer precharge denial; the actual equality adapter cold-payload control also observes zero allocations on denial. |

## Current evidence

The cold equality adapter, actual map/sequence native join and model enum native closure controls were source-reviewed and run with exact individual filters; logs are in /tmp/qsl2489-stack-review. No global computed matrix pass is claimed. The matrix run was stopped after seven minutes without output.

Focused frozen-head checks: original AdapterRow output, sparse retained field index, tick complete three-resource handoff, root refinement false replay, dependency-isolated refinement false replay, hyper fairness references and two-alias model-root subjects each passed one exact test. Map/sequence actual native projection and cold equality payload tests also passed. The model-enum native closure check FAILED at monitor_root.rs:1308 (expected one Tier row; actual []); historical IR1388 gap FND001 remains still-open. These are focused checks only.
