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
conformance corpus, the gate SHALL negotiate and run each candidate backend
of each in-scope requirement item, re-check each backend's refutations by
replay, and fail when two backends give conflicting verdicts for the same
item under the
[ADR-013 O-16](../decisions/ADR-013-canonical-type-package-conversion-ownership.md)
agreement relation.

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
one entry per in-scope item and one per vector that yields no item, in
ascending order of vector id (bytewise) and then request index. An item
entry names the item's vector, request index and capability and, for each
candidate in FR-075 candidate order, its `BackendId`, its negotiated
disposition, and, when it ran, its terminal record (result value, O-16
category, proof basis, declared finite domain for a `bounded`-mode run, and
typed cause) and, for a `refuted` record, the gate's replay result. Each
item entry settles `agree`, `conflict`, `not-compared` or `tool-failure`. A
vector entry with no item settles `not-compiled`, holding the compile
refusal or limit, or `tool-failure`. The report gives the count of each
settlement.

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
`compile` with fresh accounting at the effective limits:

- If `compile` returns a checked package, then the gate SHALL build its
  request through the FR-075 request builder, naming no backend. An item
  is in scope when that candidate set holds two or more backends.
- If `compile` returns a refusal or reaches a limit, then the vector SHALL
  yield no item and its entry SHALL be `not-compiled`, holding the
  refusal or the limit. This is the expected result of the corpus's
  refusal, just-outside and incomplete vectors, so it fails nothing.
- If `compile` panics, returns an internal fault, or cannot read a file
  the vector names, then the vector's entry SHALL be `tool-failure`,
  holding the panic message, the fault or the I/O error,
  and the gate SHALL continue with the next vector.

### Negotiation

For each candidate of an in-scope item, the gate SHALL build the request
again naming that one `BackendId`, so the FR-075 candidate set is that one
backend, and SHALL have it negotiated (ADR-012 §7.2 step 3). The gate SHALL
run, with fresh resource accounting at the effective limits, only the
candidates settled `supported`. A candidate settled `requires-bound`,
`unsupported` or `invalid-request` SHALL NOT run; its entry holds the
disposition and its O-16 category (`unsupported`, `unsupported`,
`refusal`), which is not definite. The in-scope count is taken before
negotiation; an item with fewer than two candidates settled `supported`
is `not-compared`.

### What a backend returns, and the replay re-check

A backend that runs SHALL return its FR-331 terminal record, already
settled by its provider under ADR-013 C-09: for a counterexample, the
orchestrating driver (ADR-011 T-13) has run its E9 replay. The gate does
not settle a raw backend outcome.

The gate SHALL then replay the counterexample of every `refuted` record
again through the `replay` facade, as a re-check of the provider. If that
replay settles `ReproducedWithEvaluatedWitness`, then the record stays
`refuted`. If it settles `Inconclusive` (the replayed verdict differs, or
the replay completed no value), then the record SHALL count as not
definite, and the item SHALL be `conflict`, naming the backend, its
`refuted` record and the gate's replay result: the backend and the
evaluator disagree about a definite verdict. If the re-check replay
refuses (a `ReplayRefusal` other than a fault, such as a replay limit
reached), then the record SHALL count as not definite, as O-16's
`replay_refused` does, and the entry SHALL hold the refusal and its catalog
code; it fails nothing. If the re-check replay faults (`ReplayRefusal::Fault`
or an admission `InternalFault`), then the record SHALL count as not
definite and the item SHALL be `tool-failure`, holding the fault, as O-16
maps a replay fault to `failed`. A record the provider
settled `inconclusive` with `replay_parity` or `replay_refused` is not
definite and is reported with its cause; it fails nothing.

### Agreement

The gate SHALL settle each in-scope item by the O-16 agreement relation
over the terminal records of its candidates that ran:

- `conflict` when one record is `proved` and another is `refuted`, and the
  two are comparable under O-16, or when a replay re-check fails;
- `tool-failure` when no pair conflicts and a backend run panicked,
  returned an internal fault or a `failed` terminal record, or a re-check
  replay faulted, holding the panic message or the fault;
- `agree` when neither holds and at least two records are definite
  (`proved` or `refuted`) and comparable;
- `not-compared` otherwise. The entry names, for each record, why it was
  not compared: its candidate did not negotiate `supported`, its re-check
  replay refused (naming the refusal), its record is
  not definite (its O-16 category), or a `bounded`-mode proof met a
  counterexample whose witness lies outside its declared domain (naming
  the domain).

A `conflict` entry SHALL name the item and both sides with their records.

### Preflight refusal

Before any backend runs, the gate SHALL validate its arguments, the corpus
and the scope as FR-353 does, and SHALL check that each backend it will
run can run: its provider is registered and the tool it calls (Kani, an
SMT solver) is installed and reports a version. If any check fails, then
the gate SHALL report the refusal, naming its cause and locus (for a
missing tool, the `BackendId` and the tool), run no backend and exit 2. If,
after compiling and building the requests and before any backend runs, no
item in the scope has two or more candidates, then the gate SHALL refuse
with cause `no-comparable-item` and exit 2.

### Exit contract

| Exit | Meaning |
| --- | --- |
| 0 | the run completed with no `conflict` and no `tool-failure` entry |
| 1 | the run completed with at least one `conflict` or `tool-failure` entry |
| 2 | refused before any backend runs: no settlement |

`not-compiled` and `not-compared` entries change no exit status. A
`tool-failure` changes no other entry's settlement.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-448-AC-1 | Over a fixture corpus with one item that two registered backends both prove and one item that both refute with a counterexample that replays `ReproducedWithEvaluatedWitness`, both items are `agree`, the report names each backend's disposition and terminal record and the gate's replay result for each refutation, and the gate exits 0. | Test |
| FR-448-AC-2 | With a test backend registered that answers `proved` for the item the other backend refutes, that item is `conflict`, the report names the item, both `BackendId`s and both terminal records, and the gate exits 1; the agreeing item stays `agree`. | Test |
| FR-448-AC-3 | A `BoundedComplete{depth: 3}` proof against a reproduced counterexample is `conflict` and exits 1. A `bounded`-mode proof over the declared domain `0..=10` against a reproduced counterexample with witness 12 is `not-compared`, naming the domain, and exits 0; with witness 4 it is `conflict` and exits 1. A `proved` record against an `inconclusive` (including `BoundReached`), `incomplete`, `unsupported` or `tested` record is `not-compared`, naming that record's category, and exits 0. | Test |
| FR-448-AC-4 | A test backend that returns a `refuted` record whose counterexample the gate's replay does not reproduce makes the item `conflict`, naming that backend, its record and the replay result, and the gate exits 1. A test backend whose provider settled `inconclusive` with `replay_parity` leaves the item `not-compared` against another backend's `proved`, the entry holding that cause, and the gate exits 0. | Test |
| FR-448-AC-5 | A run with a candidate whose tool is not installed refuses naming that `BackendId` and the tool; a corpus whose in-scope items each have one candidate refuses with `no-comparable-item`; a malformed `--limit` and an unknown `--capability` each refuse naming the operand. Each exits 2 and runs no backend. | Test |
| FR-448-AC-6 | Two runs over the AC-1 corpus give byte-equal JSON reports, with entries in ascending bytewise vector id and then request index order. | Test |
| FR-448-AC-7 | A candidate whose negotiation settles `requires-bound` does not run and is reported with that disposition; with one other candidate settled `supported`, the item is `not-compared`. | Test |
| FR-448-AC-8 | Adding a vector whose `compile` refuses `ill_typed` yields a `not-compiled` entry holding the refusal and leaves the exit status at 0. A vector naming a source file absent from the corpus yields a `tool-failure` entry holding the I/O error, the other entries keep their settlements, and the gate exits 1. | Test |
| FR-448-AC-9 | A `refuted` record whose re-check replay is run under a replay limit it reaches is reported not definite, holding the replay refusal and its catalog code; against another backend's `proved` the item is `not-compared` and the gate exits 0. A `refuted` record whose re-check replay faults makes the item `tool-failure`, holding the fault, and the gate exits 1. | Test |

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
