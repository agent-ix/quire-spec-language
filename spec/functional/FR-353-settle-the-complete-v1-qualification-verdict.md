---
id: FR-353
title: "Settle the complete-V1 qualification verdict"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-035
    type: implements
  - target: ix://agent-ix/quire-spec-language/FR-351
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-352
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-311
    type: depends_on
---
# FR-353: Settle the complete-V1 qualification verdict

## Description

When a conformance run completes, the runner SHALL report QSL qualified for
the run's scope if and only if every capability in that scope is `passed`
(FR-351), and SHALL exit 0 when qualified and non-zero otherwise.

Complete-V1 qualified is that verdict over the default scope: every
capability the corpus lists for a QSL subject.

## Inputs

The capability outcomes of FR-351 and the scope.

## Outputs

A verdict in the FR-352 report: `qualified` or `not-qualified`, with the
scope it covers, and the process exit status.

## Behavior

- If every in-scope capability is `passed`, then the verdict SHALL be
  `qualified` and the exit status 0.
- If any in-scope capability is not `passed`, then the runner SHALL report
  `not-qualified`, naming every such capability with its outcome, and SHALL
  exit 1.
- If the run is refused before any vector executes (FR-351, an unreadable
  corpus), then the runner SHALL report the refusal and exit 2, with no
  verdict.
- A verdict for a caller-selected scope SHALL name that scope. Only a run
  over the default scope reports `complete-v1: qualified`.
- A non-passing capability outside the scope changes no verdict.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-353-AC-1 | A fixture corpus whose in-scope vectors all pass, run over the default scope, reports `complete-v1: qualified` and exits 0. | Test (TC-880) |
| FR-353-AC-2 | The same corpus with one vector's expected cause changed reports `not-qualified`, naming that vector's capability as `failed`, and exits 1; the same corpus with one capability's vectors removed reports `not-qualified`, naming it `uncovered`. | Test (TC-880) |
| FR-353-AC-3 | A run scoped to the capabilities that pass, over the corpus of AC-2, reports `qualified` for that named scope, does not report `complete-v1: qualified`, and exits 0. | Test (TC-881) |
| FR-353-AC-4 | A run over a corpus location that does not exist reports the I/O refusal, gives no verdict and exits 2. | Test (TC-881) |

## Dependencies

- [FR-351](FR-351-settle-each-capability-from-its-executed-vectors.md),
  [FR-352](FR-352-report-capability-coverage.md).
- QSpec FR-311 (a gap prevents only its containing scope from claiming
  complete success, FR-311-AC-2).
