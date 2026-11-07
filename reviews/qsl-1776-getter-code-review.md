---
id: SR-20261105101094
title: QSL-1776 getter follow-up code-review
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; QSL-1776 PR2475; caller-budget diagnostic adoption; FR-531; TC-1331"
review_set: subset
---

## Summary

Owner NativeJoinBudget getters supply original limits. ExactTypeProjectionError.configured_bound is preserved when present. with_budget wraps resource refusals once, retaining typed Projection/Equality causes; semantic errors remain unchanged. Actual is cumulative consumed plus first unpaid amount in u128 with saturating addition. Bare admission and both evidence constructors attach the diagnostic; report error migration retains it. Public change is intentional, documented, and all in-repo exhaustive consumers updated. No unsafe, wire format, CI, shim, mirror carrier, manifest pin or authority substitution added.

## Verdict

PASS: no findings in changed scope, subject to the code lead's final combined-tree gate.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Coverage

Reviewed origin/main...HEAD in the one ticket worktree; Rust review folded into code-review. No applicable AssuranceProfile found. Base plus integrity selected for explanatory contract/procedure edit; no new relationships or objects. Plan completion: not assessed. Full aggregate gate reserved for the lead. Focused all-target checked-handoff compile passed in /tmp/QSL-1776-getter-check.log; focused runtime evidence is recorded below.

## Examined scope

```yaml
{
  "scope": [
    {
      "id": "FR-531-AC-21",
      "path": "spec/functional/FR-531-evaluate-from-a-checked-package-read-from-its-bytes-alone.md",
      "role": "examined",
      "excerpt": "A `BarePackage::with_supply` over a fixture whose identity join is measured on a fresh budget at work **W** and storage **S** returns the package on `NativeJoinBudget::new(W, S)`, and returns `ExhaustedWork` on `new(W - 1, S)` and `ExhaustedStorage` on `new(W, S - 1)`, each with IR's locus and attempted charge, no package, the work denial naming `i2.work_units` and the storage denial naming `i2.native_join_storage`, and the budget's consumed counters equal to those IR reports. The three results differ in kind from `UnknownIdentityWitness`, `InternalFault` and an empty identity set."
    },
    {
      "id": "FR-531-AC-24",
      "path": "spec/functional/FR-531-evaluate-from-a-checked-package-read-from-its-bytes-alone.md",
      "role": "examined",
      "excerpt": "`V2ReadLimits` has a required `storage` member and no `Default`; a read's evaluation view over limits whose `storage` is exactly the measured join storage succeeds and over one unit less returns `ViewFailure::Exhausted` with `ExhaustedStorage`, whose refusal names `i2.native_join_storage`; `EvidencePackage::from_read` and `from_emission` over a budget one unit short of the source join return `PackageMismatch::Exhausted`; the handoff writer over `native_join` one unit short returns `Error::NativeJoinExhausted` and writes no directory."
    },
    {
      "id": "FR-531-AC-25",
      "path": "spec/functional/FR-531-evaluate-from-a-checked-package-read-from-its-bytes-alone.md",
      "role": "context_only",
      "excerpt": "An inspection of the public signatures of `selected_exact_identities_from_rows`, `SelectedModels::exact_identities`, `CheckedGraph::selected_exact_identities`, `ReadIdentityRows::selected`, `BarePackage::with_supply` and the evidence constructors finds each takes a `NativeJoinBudget` by mutable reference, finds no `NativeJoinBudget::new` and no `u64::MAX` allowance in a library path below an entry, and finds that `BarePackage` implements neither `Copy` nor `Clone`."
    },
    {
      "id": "FR-531",
      "path": "spec/functional/FR-531-evaluate-from-a-checked-package-read-from-its-bytes-alone.md",
      "role": "examined",
      "excerpt": "Every entry, including `with_supply`, `from_read` and `from_emission`\n  with an opaque caller budget, retains the native error together with a\n  `NativeJoinLimitError`: its `limit` is the diagnostic\n  `LimitExceeded` with the original configured bound and the cumulative\n  attempted count (consumed plus the first unpaid charge), and its catalog\n  fields include the setting."
    },
    {
      "id": "TC-1331",
      "path": "spec/test-cases/TC-1331-evaluate-from-a-checked-package-read-from-its-bytes-alone.md",
      "role": "examined",
      "excerpt": "opaque-budget denials preserve the original configured bound, native locus and setting, and actual is cumulative consumed plus first unpaid charge"
    },
    {
      "id": "FR-531-AC-28",
      "path": "spec/functional/FR-531-evaluate-from-a-checked-package-read-from-its-bytes-alone.md",
      "role": "examined",
      "excerpt": "Producer and bytes-only equality admission pass the caller's same `NativeJoinBudget` through IR's admitted equality reader after identity/type projection. Decisions and non-resource witness refusals are retained for later unmetered selection and projection. Successful reads consume native work and storage cumulatively; exact measured bounds admit, and one unit less in either resource refuses with the original typed IR equality denial and recorded first unpaid charge, never an unknown witness or internal fault. Reusing an admitted slot consumes no additional native allowance."
    }
  ]
}
```

## Current evidence

Reviewed final source be7850179, including detached-limit API preservation and equality diagnostics assertions. Correct feature-enabled bare_package focused suite passed 27 tests, zero failures (/tmp/QSL-1776-getter-checked-tests.log); equality-budget suite passed 3 tests, zero failures (/tmp/QSL-1776-getter-equality.log). Initial no-feature invocation selected zero tests and carries no runtime credit. Direct bare paired trace tags bind new genuine test to FR-531-AC-21/24, and the equality test to AC-28 by manual source inspection. Full computed matrix command has been CPU-active over six minutes and has emitted no result at review closure; computed matrix evidence is unavailable in this pass, not claimed green. Manual criterion/test/code correspondence verified. Focused package/eval all-target check passed. Final workspace gate remains the lead's responsibility.

The incomplete full-matrix process was stopped after the bounded review; no result or coverage credit inferred. The targeted manual trace assessment is the gap evidence for this PR.
