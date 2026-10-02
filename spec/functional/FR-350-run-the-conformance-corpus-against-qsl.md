---
id: FR-350
title: "Run the complete-V1 conformance corpus against QSL"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-035
    type: implements
  - target: ix://agent-ix/quire-specification/FR-311
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-336
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-096
    type: depends_on
---
# FR-350: Run the complete-V1 conformance corpus against QSL

## Description

When a maintainer runs the `qualify` repository gate (FR-353) over a
conformance corpus,
the runner SHALL execute every vector whose applicability names a QSL
subject through the QSL public entry point for that subject, and SHALL
compare the typed result with the vector's expected disposition.

The corpus is QSpec's (QSpec FR-336, interface_019). QSpec owns its vector
shape, its classes (positive, boundary, just-outside, refusal, incomplete,
metamorphic) and each vector's expected disposition. This requirement
specifies how QSL executes and compares them.

## Inputs

- The corpus, read from the location the caller names (the QSpec
  repository's published corpus). The runner reads the corpus in place.
- An optional capability scope (FR-351).
- Resource limits for each vector: the stage limits (FR-096) and the
  evaluation budget the compiler and runtime already take, each with its
  published default, each raisable by the caller.

## Outputs

One outcome for each executed vector: `passed`, `failed`, `unsupported`,
`incomplete` or `tool-failure`, each holding the vector id, its capability
id and, unless `passed`, the expected and actual typed result. `incomplete`
and `tool-failure` are QSpec FR-311 `unexecuted` results, kept apart so the
report names why the vector did not run to a result.

## Behavior

### Subjects and applicability

The QSL subjects are three QSpec FR-311 consumer types, each reached through
the entry point a caller of that subject uses:

| Consumer type | QSL subject | Entry point |
| --- | --- | --- |
| `language` | the compiler (S0 to S4 and the checked package) | `compile` |
| `runtime` | the reference runtime, and the replay facade for a vector whose input is a replay request | `run`, `replay` |
| `tool` | the formatter | `format` |

QSL reads QSpec FR-336 `applicability` at two levels. A capability is in
the default scope (FR-351) when its applicability names a QSL consumer type
and a semantic profile QSL's definition catalog holds. A vector SHALL run
when its own applicability names a QSL consumer type, through that type's
entry point. The runner calls no internal function a caller cannot reach.

### Settlement

If executing a vector panics, returns an internal fault, or cannot read a
file the vector names, then its outcome SHALL be `tool-failure`, holding
the panic message, the fault or the I/O error. Otherwise the runner SHALL
settle the vector by the first of these, in this order, that holds:

1. `passed`: the actual result matches the expected disposition. A value,
   verdict or checked package identity matches when it is equal. A refusal
   matches when it has the same catalog code and cause, and the same locus
   when the vector names one. This includes an expected limit refusal
   (a boundary or `incomplete`-class vector) and an expected
   `unknown_required_feature` or `unsupported_projection` refusal.
2. `unsupported`: QSL returned `unsupported_projection` or
   `unknown_required_feature`, holding the code and cause QSL returned.
3. `incomplete`: the vector reached a resource limit, naming the limit, its
   value and the option that raises it.
4. `failed`: any other result, holding the expected and the actual result.
   A refusal with the right code and a different cause is `failed`, as is a
   success where a refusal was expected and the reverse.

### Isolation

The runner SHALL settle each vector independently: a `tool-failure`,
`incomplete` or `failed` vector SHALL change no other vector's outcome, and
the runner SHALL continue with the next vector.

### Determinism

The runner SHALL give the same outcome for each vector on every run over
the same corpus with the same QSL build and limits. It reads no clock,
environment variable or search path to decide an outcome.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-350-AC-1 | Over a fixture corpus holding one positive compile vector, one positive `run` vector, one `replay` vector and one `format` vector, each with the expected result QSL returns, each vector is `passed` and each ran through its subject's public entry point. | Test (TC-875) |
| FR-350-AC-2 | A refusal vector expecting `ill_typed`/`unit-mismatch` is `passed` when QSL returns that pair, and `failed` when QSL returns `ill_typed`/`type-mismatch`, the failure holding the expected and the actual code and cause. A refusal vector over source QSL admits is `failed`, holding the expected refusal and the actual success. | Test (TC-875) |
| FR-350-AC-3 | A vector that expects a success, which QSL answers with `unknown_required_feature`/`unknown-feature`, is `unsupported`, holding that pair; the same answer to a vector that expects that pair is `passed`. A vector that expects a success, run with an evaluation budget of zero, is `incomplete`, naming the budget, its value and the option that raises it, and with the default budget is `passed`. A boundary vector that expects the zero-budget limit refusal, run with a budget of zero, is `passed`. A vector expecting `ill_typed`/`type-mismatch` that QSL answers with `unknown_required_feature` is `unsupported`. | Test (TC-876) |
| FR-350-AC-4 | A vector naming a source file absent from the corpus is `tool-failure` holding the I/O error, and the vectors before and after it in the corpus keep the outcomes they have when it is removed. | Test (TC-876) |
| FR-350-AC-5 | Two runs over the same fixture corpus with the same build and limits give equal per-vector outcomes. | Test (TC-876) |

## Dependencies

- QSpec FR-311 (qualification behaviour), FR-336 and interface_019 (the
  corpus and its vectors).
- [FR-096](FR-096-stage-limits-refusal-records-and-readers-carry-a-locus.md)
  (stage limits and catalog-coded refusals the comparison reads).
- [FR-351](FR-351-settle-each-capability-from-its-executed-vectors.md)
  settles each capability from these outcomes.
