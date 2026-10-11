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
- each capability's outcome and its deciding vector ids, and, for each
  non-passing capability, every non-passing vector with its own outcome,
  expected result and actual result. A vector with a lower settlement
  precedence remains visible even when it did not decide the capability.

## Behavior

- The report SHALL list capabilities in ascending capability-id order and,
  inside each capability, vectors in ascending vector-id order.
- The report SHALL be equal across two runs over the same corpus with the
  same build, limits and scope.
- An `uncovered` capability SHALL count in the in-scope total and not in
  the `passed` count.
- If the resolved scope is empty, then the runner SHALL emit only the
  [FR-353 preflight refusal](FR-353-settle-the-complete-v1-qualification-verdict.md)
  without a coverage report or percentage, in either output mode. The
  coverage division applies only to completed runs with a nonempty scope;
  an empty scope has neither zero-percent nor hundred-percent coverage.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-352-AC-1 | Over a fixture corpus with four in-scope capabilities, two `passed`, one `failed` and one `uncovered`, the report gives counts 2, 1, 0, 0, 0, 1 for `passed`, `failed`, `tool-failure`, `unsupported`, `incomplete` and `uncovered`, and coverage 50 per cent. When a valid corpus resolves to an empty default scope, both JSON and text modes emit the empty-scope refusal with no coverage report or percentage. | Test |
| FR-352-AC-2 | The `failed` capability's entry holds its failing vector's id with the expected and the actual result; the `passed` capabilities' entries hold no vector detail. Adding a `tool-failure`, `unsupported` and `incomplete` vector to the failed capability leaves it `failed` with the same deciding vector id, while the report retains all four non-passing vectors with their own outcomes and expected and actual results. | Test |
| FR-352-AC-3 | The JSON report of two runs over the fixture corpus is byte-equal, capabilities and vectors are in ascending id order, and the report names the QSL version that ran. The text report holds the same counts and coverage. | Test |

## Dependencies

- [FR-351](FR-351-settle-each-capability-from-its-executed-vectors.md)
  (capability outcomes).
- QSpec FR-311 (per-capability results and coverage report).
