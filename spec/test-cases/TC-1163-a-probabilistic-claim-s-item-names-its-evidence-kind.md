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
   claim's requirement record and its normal form.
2. Register an EN-4 manifest advertising `probabilistic-satisfaction` with
   `{statistical}` and an EN-5 manifest with `{exact}`.
3. Build the request items with no evidence selection, with the selection
   `exact`, and with both kinds; compute each item's candidates.

Tag the tests `#[trace("TC-1163", "<criterion>")]`.

## Expected Results

- Step 1: `FairTerminates` and the `Reach` claim record `exact`, `P95`
  records `statistical`; the evidence kind is not part of the normal form.
- Step 3: with no selection, `P95`'s item names `statistical` (candidate
  EN-4) and `FairTerminates`'s names `exact` (candidate EN-5); with `exact`,
  every probabilistic item names `exact`; with both, each probabilistic
  claim gives an `exact` then a `statistical` item at consecutive request
  indices with one occurrence key; the `value-validity` item carries no
  evidence kind in every run; no item settles `absent-evidence-kind` or
  `unknown-evidence-kind`.
