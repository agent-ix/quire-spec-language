---
id: FR-448
title: "Cross-check verification backends over the conformance corpus"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-035
    type: implements
  - target: ix://agent-ix/quire-spec-language/FR-075
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-072
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-350
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-018
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-353
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-402
    type: traces_to
---
# FR-448: Cross-check verification backends over the conformance corpus

## Description

When a maintainer runs the `backend-agreement` repository gate over a
conformance corpus, the gate SHALL run every candidate backend of each
in-scope requirement item, settle each backend's result to its
[ADR-013 O-16](../decisions/ADR-013-canonical-type-package-conversion-ownership.md)
terminal record, and fail when two backends give conflicting verdicts for
the same item.

Two verification backends that both answer an item and disagree mean that
at least one of them returns a wrong verdict. The qualification gate
([FR-353](FR-353-settle-the-complete-v1-qualification-verdict.md)) compares
QSL with the corpus's expected dispositions; this gate compares QSL's
backends with one another, including items whose expected disposition the
corpus does not record. The verdict agreement relation is ADR-013 O-16's
"Agreement across backends".

## Inputs

- The QSpec conformance corpus, read in place, as for FR-350 (`--corpus`).
- An optional capability scope, resolved as FR-351 resolves it
  (`--capability`, repeatable).
- Resource limits, resolved through the
  [FR-255 settings operation](FR-255-name-the-setting-that-raises-a-reached-limit.md)
  (`--limit`, repeatable), applied to every backend run.
- The gate's backend registry ([FR-075](FR-075-compute-candidates-from-registered-backends.md)).

## Outputs

A JSON report (text with `--text`) and the exit status. The report holds
one entry per in-scope item, in ascending order of vector id and then
request index, naming the item's vector, request index, capability and,
for each candidate backend in FR-075 candidate order, its `BackendId`, its
terminal record (result value, O-16 category, proof basis or bound, and
typed cause), and, for a counterexample, its replay result. Each entry
settles `agree`, `conflict` or `not-compared`. The report also gives the
count of each settlement.

## Behavior

### Invocation

```text
cargo run --package xtask -- backend-agreement --corpus <dir> [--capability <id>]... [--limit <setting>=<value>]... [--text]
make backend-agreement CORPUS=<dir>
```

The gate runs beside `qualify`. `make ci` does not run it, because `make ci`
requires no backend tool such as Kani or an SMT solver.

### Scope

For each vector in the selected scope whose applicability names the
`language` consumer type, the gate SHALL compile the vector's source through
`compile` and build its request through the FR-075 request builder. An
item is in scope when its FR-075 candidate set holds two or more backends.
The gate SHALL run every candidate of each in-scope item, each with fresh
resource accounting at the effective limits.

### Replay

The gate SHALL replay every counterexample a backend returns through the
`replay` facade and settle it under ADR-013 O-16 and ADR-018 SM-7: only a
counterexample whose replay settles `ReproducedWithEvaluatedWitness`
becomes `refuted`. A counterexample whose replay disagrees, refuses or
faults keeps the terminal record O-16 gives it (`inconclusive` with
`replay_parity` or `replay_refused`, or `failed`), and the report names
the replay result.

### Agreement

The gate SHALL settle each in-scope item by the O-16 agreement relation
over its candidates' terminal records:

- `conflict` when one backend's record is `proved` and another's is
  `refuted`, and the two are comparable under O-16;
- `agree` when no pair conflicts and at least two records are definite
  (`proved` or `refuted`) and comparable;
- `not-compared` otherwise. The entry names, for each record, why it was
  not compared: not definite (its O-16 category), or a bounded `proved`
  record against a record at a different bound or an unbounded one.

A `conflict` entry SHALL name the item and both backends with their
terminal records.

### Preflight refusal

Before any backend runs, the gate SHALL validate its arguments, the corpus
and the scope as FR-353 does, and SHALL check that each backend it will
run can run: its provider is registered and the tool it calls (Kani, an
SMT solver) is installed and reports a version. If any check fails, then
the gate SHALL report the refusal, naming its cause and locus (for a
missing tool, the `BackendId` and the tool), run no backend and exit 2. If
no item in the scope has two or more candidates, then the gate SHALL refuse
with cause `no-comparable-item` and exit 2.

### Exit contract

| Exit | Meaning |
| --- | --- |
| 0 | the run completed and no item is `conflict` |
| 1 | the run completed and at least one item is `conflict` |
| 2 | refused before any backend runs: no settlement |

`not-compared` entries change no exit status. A backend run that panics or
returns an internal fault is recorded with its `failed` terminal record and
changes no other item's settlement.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-448-AC-1 | Over a fixture corpus with one item that two registered backends both prove and one item that both refute with a counterexample that replays `ReproducedWithEvaluatedWitness`, both items are `agree`, the report names each backend's terminal record, and the gate exits 0. | Test |
| FR-448-AC-2 | With a test backend registered that answers `proved` for the item the other backend refutes, that item is `conflict`, the report names the item, both `BackendId`s and both terminal records, and the gate exits 1; the agreeing item stays `agree`. | Test |
| FR-448-AC-3 | A `BoundedComplete{depth: 3}` proof against a refutation produced at bound 5 is `not-compared`, naming the differing bound, and exits 0; the same proof against a refutation produced at bound 3 is `conflict` and exits 1, and an unbounded proof against the bound-5 refutation is `conflict` and exits 1. A `proved` record against an `inconclusive`, `incomplete`, `unsupported` or `tested` record is `not-compared`, naming that record's category, and exits 0. | Test |
| FR-448-AC-4 | A test backend that returns a counterexample whose replay does not reproduce settles `inconclusive` with `replay_parity`, never `refuted`; against another backend's `proved` the item is `not-compared`, and the report holds the replay result. | Test |
| FR-448-AC-5 | A run with a candidate whose tool is not installed refuses naming that `BackendId` and the tool; a corpus whose in-scope items each have one candidate refuses with `no-comparable-item`; a malformed `--limit` and an unknown `--capability` each refuse naming the operand. Each exits 2 and runs no backend. | Test |
| FR-448-AC-6 | Two runs over the AC-1 corpus give byte-equal JSON reports, with entries in ascending vector id and request index order. | Test |

## Dependencies

- [FR-075](FR-075-compute-candidates-from-registered-backends.md): the
  candidate set that decides which backends answer an item.
- [FR-072](FR-072-implement-typed-replay-result.md) and ADR-018 SM-7: the
  replay that settles a counterexample.
- ADR-013 O-16: the terminal records and the agreement relation.
- [FR-350](FR-350-run-the-conformance-corpus-against-qsl.md),
  [FR-351](FR-351-settle-each-capability-from-its-executed-vectors.md),
  [FR-353](FR-353-settle-the-complete-v1-qualification-verdict.md): the
  corpus, scope and preflight it shares with `qualify`.
