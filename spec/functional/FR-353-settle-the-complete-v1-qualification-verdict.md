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

When the `qualify` repository gate finishes a conformance run, the gate SHALL
report QSL qualified for the run's scope if and only if every capability in
that scope is `passed` (FR-351).

Complete-V1 qualified is that verdict over the default scope: every
capability the corpus lists for a QSL consumer type (FR-350).

## Inputs

The capability outcomes of FR-351 and the scope.

## Outputs

A verdict in the FR-352 report: `qualified` or `not-qualified`, with the
scope it covers, and the gate's exit status.

## Behavior

### Invocation

`qualify` is an xtask repository gate, run beside the repository's other
xtask gates:

```text
cargo run --package xtask -- qualify --corpus <dir> [--capability <id>]... [--limit <setting>=<value>]... [--text]
make qualify CORPUS=<dir>
```

`--corpus` names the QSpec conformance corpus, read in place (FR-350).
Each `--capability` adds one capability id to a caller-selected scope;
with none, the scope is the default scope. Each `--limit` raises or lowers
one FR-255 setting. `--text` writes the FR-352 report as text instead of
JSON.

### Exit contract

| Exit | Meaning |
| --- | --- |
| 0 | `qualified` for the run's scope |
| 1 | `not-qualified`: at least one in-scope capability is not `passed` |
| 2 | refused before any vector executes: no verdict |

- If every in-scope capability is `passed`, then the gate SHALL report
  `qualified` and exit 0.
- If any in-scope capability is not `passed`, then the gate SHALL report
  `not-qualified`, naming every such capability with its outcome, and SHALL
  exit 1.
- If the corpus location cannot be read, if the corpus fails QSpec FR-336's
  validation (a `contract_version` other than `quire.conformance-corpus/v1`,
  a missing required property, a duplicate key, an out-of-domain value or a
  non-canonical encoding), if a `--capability` id names no capability the
  corpus lists (FR-351), or if an argument is malformed, then the gate SHALL
  report the refusal, naming its cause and locus, give no verdict and exit 2.
- A verdict for a caller-selected scope SHALL name that scope. Only a run
  over the default scope SHALL report `complete-v1: qualified`.
- A non-passing capability outside the scope SHALL change no verdict.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-353-AC-1 | A fixture corpus whose in-scope vectors all pass, run over the default scope, reports `complete-v1: qualified` and exits 0. | Test (TC-880) |
| FR-353-AC-2 | The same corpus with one vector's expected cause changed reports `not-qualified`, naming that vector's capability as `failed`, and exits 1; the same corpus with one capability's vectors removed reports `not-qualified`, naming it `uncovered`. | Test (TC-880) |
| FR-353-AC-3 | A run scoped to the capabilities that pass, over the corpus of AC-2, reports `qualified` for that named scope, does not report `complete-v1: qualified`, and exits 0. | Test (TC-881) |
| FR-353-AC-4 | A run over a corpus location that does not exist reports the I/O refusal, gives no verdict and exits 2; so does a run over a corpus whose `contract_version` is `quire.conformance-corpus/v0`, naming `/contract_version`, a corpus with one vector missing its expected disposition, naming that member, and a run with `--capability CAP-Z` that the corpus does not list, naming `CAP-Z`. No vector executes in any of them. | Test (TC-881) |

## Dependencies

- [FR-351](FR-351-settle-each-capability-from-its-executed-vectors.md),
  [FR-352](FR-352-report-capability-coverage.md).
- QSpec FR-311 (a gap prevents only its containing scope from claiming
  complete success, FR-311-AC-2).
