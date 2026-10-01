---
id: FR-351
title: "Settle each capability from its executed vectors"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-035
    type: implements
  - target: ix://agent-ix/quire-spec-language/FR-350
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-311
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-336
    type: depends_on
---
# FR-351: Settle each capability from its executed vectors

## Description

When a conformance run completes, the runner SHALL settle one outcome for
each capability in scope from the outcomes of that capability's vectors
executed in the same run (FR-350).

## Inputs

- The per-vector outcomes of FR-350.
- The scope: the capability ids the caller selects, or, when the caller
  selects none, every capability the corpus lists whose applicability names
  a QSL subject.

## Outputs

One outcome for each capability in scope: `passed`, `failed`,
`tool-failure`, `unsupported`, `incomplete` or `uncovered`, holding the ids
of the vectors that decided it.

## Behavior

- If a capability in scope has no vector that applies to a QSL subject,
  then its outcome SHALL be `uncovered`.
- Otherwise its outcome SHALL be the first of these that holds:
  1. `failed`, when any of its vectors is `failed`;
  2. `tool-failure`, when any is `tool-failure`;
  3. `unsupported`, when any is `unsupported`;
  4. `incomplete`, when any is `incomplete`;
  5. `passed`, when every one is `passed`.
- A non-`passed` capability SHALL hold the ids of every vector with that
  outcome.
- Each capability outcome is computed from vectors executed in this run.
  The corpus's recorded `results` member (QSpec FR-336) is not an input to
  any outcome.
- If the caller selects a capability id the corpus does not list, then the
  runner SHALL refuse the run with `unknown_required_feature`/
  `unknown-feature`, naming the id, before it executes any vector.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-351-AC-1 | A capability whose three vectors are `passed` is `passed`. A capability with one `passed` and one `failed` vector is `failed`, holding the failed vector's id. A capability with one `unsupported` and one `incomplete` vector is `unsupported`. A capability with one `tool-failure` and one `failed` vector is `failed`. | Test (TC-877) |
| FR-351-AC-2 | A capability the corpus lists for the language subject with no vector is `uncovered`. A capability whose applicability names no QSL subject is outside the default scope and appears in no outcome. | Test (TC-877) |
| FR-351-AC-3 | A corpus whose `results` member records `passed` for a vector QSL fails settles that vector's capability `failed`. | Test (TC-878) |
| FR-351-AC-4 | A run scoped to two capability ids settles exactly those two. A run scoped to an id the corpus does not list refuses `unknown_required_feature`/`unknown-feature` naming the id and executes no vector. | Test (TC-878) |

## Dependencies

- [FR-350](FR-350-run-the-conformance-corpus-against-qsl.md) (per-vector
  outcomes).
- QSpec FR-311 (a missing, unsupported, failed or unexecuted vector is a
  gap) and FR-336 (capability ids, applicability, the `results` member).
