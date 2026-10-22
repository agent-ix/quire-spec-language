---
id: TC-1163
title: "A probabilistic claim's request item names its evidence kind"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-075
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-195
    type: verifies
---
# TC-1163: A probabilistic claim's request item names its evidence kind

## Description

Verify that S3 records each probabilistic claim's evidence kind on its
requirement record, and that FR-075's request builder writes items with
that kind, with the one kind a request selects, or once per kind when a
request selects both, so that no item it writes lacks an evidence kind.

Scope: FR-075-AC-18, FR-195-AC-6.

## Test Procedure

1. Check a unit holding FR-186's `P95`, FR-195's `FairTerminates`, a
   `Reach` claim under a workload with confidence parameters, and one
   value function with a `value-validity` claim; read each probabilistic
   claim's requirement record (key, kind, extent, result bound, conversion
   context, path condition, evidence kind).
2. Register an EN-4 manifest advertising `probabilistic-satisfaction` with
   `{statistical}` and an EN-5 manifest with `{exact}`.
3. Over the package whose records in key order are `P95`, `FairTerminates`
   and the `value-validity` record, build the request items with no
   evidence selection, with the selection `exact`, and with both kinds;
   compute each item's candidates and `P95`'s obligation identity in each
   request.

Tag the tests `#[trace("TC-1163", "<criterion>")]`.

## Expected Results

- Step 1: one record per probabilistic claim, keyed by its `claim`
  occurrence, `probabilistic-satisfaction`, `Unbounded`, `Boolean` result
  bound, no conversion context, empty path condition; `FairTerminates` and
  the `Reach` claim record `exact`, `P95` records `statistical`.
- Step 3: no selection: three items, `P95` `statistical` (EN-4 only),
  `FairTerminates` `exact` (EN-5 only), `value-validity` at index 2 with no
  evidence kind; `exact`: three items, both probabilistic items `exact`
  with EN-5 only; both kinds: five items, indices 0 to 2 as with no
  selection, index 3 `P95` `exact` (EN-5 only), index 4 `FairTerminates`
  `statistical` (EN-4 only); `P95`'s obligation identity is equal in all
  three requests; no item settles `absent-evidence-kind` or
  `unknown-evidence-kind`.
