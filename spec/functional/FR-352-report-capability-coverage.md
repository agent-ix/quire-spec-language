---
id: FR-352
title: "Report capability coverage for a conformance run"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-035
    type: implements
  - target: ix://agent-ix/quire-spec-language/FR-351
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-311
    type: depends_on
---
# FR-352: Report capability coverage for a conformance run

## Description

When a conformance run completes, the runner SHALL write a report that
gives the capability coverage of the run, the outcome of each capability in
scope, and each non-passing vector with its expected and actual result.

Capability coverage is measured the way test coverage is: the share of
in-scope capabilities whose vectors all passed.

## Inputs

The capability outcomes of FR-351 and the vector outcomes of FR-350.

## Outputs

A report, written to standard output as JSON, or as text when the caller
asks for text. It holds:

- the QSL version that ran;
- the scope;
- the count of capabilities with each outcome, and the coverage, which is
  the `passed` count divided by the in-scope count;
- each capability's outcome and, for each non-passing capability, the
  vectors that decided it, each with its expected and actual result.

## Behavior

- The report SHALL list capabilities in ascending capability-id order and,
  inside each capability, vectors in ascending vector-id order.
- The report SHALL be equal across two runs over the same corpus with the
  same build, limits and scope.
- An `uncovered` capability SHALL count in the in-scope total and not in
  the `passed` count.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-352-AC-1 | Over a fixture corpus with four in-scope capabilities, two `passed`, one `failed` and one `uncovered`, the report gives counts 2, 1, 0, 0, 0, 1 for `passed`, `failed`, `tool-failure`, `unsupported`, `incomplete` and `uncovered`, and coverage 50 per cent. | Test (TC-879) |
| FR-352-AC-2 | The `failed` capability's entry holds its failing vector's id with the expected and the actual result; the `passed` capabilities' entries hold no vector detail. | Test (TC-879) |
| FR-352-AC-3 | The JSON report of two runs over the fixture corpus is byte-equal, capabilities and vectors are in ascending id order, and the report names the QSL version that ran. The text report holds the same counts and coverage. | Test (TC-879) |

## Dependencies

- [FR-351](FR-351-settle-each-capability-from-its-executed-vectors.md)
  (capability outcomes).
- QSpec FR-311 (per-capability results and coverage report).
