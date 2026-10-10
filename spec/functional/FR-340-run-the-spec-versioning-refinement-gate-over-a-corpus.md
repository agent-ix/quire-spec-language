---
id: FR-340
title: "Run the spec-versioning refinement gate over a corpus of revision pairs"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-034
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-017
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-001
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-056
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-099
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-106
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-109
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-115
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-342
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-343
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-344
    type: traces_to
  - target: ix://agent-ix/quire-specification/FR-290
    type: depends_on
---
# FR-340: Run the spec-versioning refinement gate over a corpus of revision pairs

## Description

QSL SHALL provide `xtask refinement versioning <corpus>`, a test gate that
checks, for every pair of revisions of one specification in a corpus, that
the superseding revision admits every case its prior revision admitted
(ADR-017 RF-2; QSpec capability row V1-TOOL-011). Each case tests one clause
implication of the QSpec FR-290 claim-form row for refinement between two
operation contracts or state models, on one input (ADR-017 RF-1).

The gate is a behavioural comparison over what each revision does on the
same input. It compiles both revisions in the running build, runs each case
through the spine against each, classifies each run (FR-342), compares the
two classes (FR-343) and reports (FR-344). It records no requirement
record, requests no backend, emits no witness envelope and has no
`Relation` family hook. A passing gate is evidence over its corpus only and
never settles `proved` for any claim.

## Inputs

- `<corpus>`: one directory. Each immediate subdirectory is one pair,
  holding a `pair.json` file and the files it names.
- `pair.json`, a JSON object with exactly these members:
  - `prior` and `superseding`: each `{path, authority, identity}`, a unit
    source and its two FR-001 labels;
  - `packages`: the domain package files the pair's units select (FR-056's
    package input), shared by both revisions;
  - `dependencies`: the FR-099 dependency input files, shared by both
    revisions;
  - `cases`: a non-empty list. Each case holds one FR-109 selection
    (`Clause`, `Function` or `Frame`, with the members FR-109 and FR-115
    give it), the snapshot and invocation files it reads (FR-106), and
    optionally any of the four limit sets `run_clause` takes (`limits`,
    `observation_limits`, `model_limits`, `accounting`).

  Every file is named by a path relative to the pair's directory.

## Outputs

One case result per case, and one tool-failure result per malformed pair,
passed to FR-344's report.

## Behavior

### Reading a pair

- If `<corpus>` cannot be read as a directory, or holds no pair
  subdirectory, then the gate SHALL record one tool-failure result naming
  the corpus path and the defect. A corpus with zero pairs never reports
  success.
- The gate SHALL read every byte a pair's runs use from the files its
  `pair.json` names, and SHALL read no environment variable, clock, search
  location or path outside the pair's directory.
- If a `pair.json` cannot be read, is not a JSON object, holds a member this
  requirement does not list or lacks one it lists, or names a path that is
  absolute or resolves outside the pair's directory, then the gate SHALL
  record one tool-failure result for that pair, naming the `pair.json` path
  and the defect, run none of its cases, and continue with the other pairs.
- If the `prior` and `superseding` sources differ in `authority` or
  `identity`, then the gate SHALL record one tool-failure result for that
  pair, naming the `pair.json` path and both label pairs. The two units are
  revisions of one specification.
- The pair's direction SHALL be the one its members name: `prior` is the
  prior revision and `superseding` the superseding one. The gate SHALL read
  no order from the revision labels and compare nothing about them.

### Running a case

- For each case, the gate SHALL call `qsl_replay::spine::run_clause` twice,
  once with the `prior` source and once with the `superseding` source, each
  time with the pair's packages and dependency input, the case's selection
  and provisions, and no expected `package_id`.
- Each call SHALL use the case's stated limit sets and, for each set the
  case does not state, that set's published default. A case's limits are
  caller-configurable resource limits; the gate imposes none of its own.
- Each case SHALL be named in every result by the pair's two
  `RawSourceRef`s (ADR-013 O-12) and its selection, never by a display
  string or a directory name.

### Where it runs

- The make target `refinement-versioning`, which `make ci` runs, SHALL run
  `cargo run --package xtask -- refinement versioning
  tests/fixtures/refinement/versioning/` over the real corpus and SHALL
  fail when the gate's exit code is not 0 (FR-344 "Invocation and exit").

## Constraints

| ID | Constraint | Type | Validation |
| --- | --- | --- | --- |
| FR-340-CON-1 | The gate reaches compile and evaluation only through `qsl_replay::spine::run_clause` and the FR-278 composition; it calls no family checker, backend or router. | Design | Inspection |

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-340-AC-1 | A corpus holding one well-formed ConfigVersion pair with two `Clause` cases runs each case against both revisions with no expected `package_id`, and FR-344's report holds exactly two case results, each naming the pair's prior and superseding `RawSourceRef`s and its selection. | Test (TC-860) |
| FR-340-AC-2 | In a corpus of one well-formed pair and six malformed ones (a `pair.json` that is not JSON; one missing `superseding`; one with an extra member `expected`; one naming `../outside.qsl`; one naming an absolute path; one whose two sources differ in `identity`), each malformed pair yields exactly one tool-failure result naming its `pair.json` path and its defect, none of its cases runs, and the well-formed pair's cases still run. A corpus directory with no pair subdirectory yields one tool-failure result naming the corpus path; verdict tool failure, exit 30. | Test (TC-860) |
| FR-340-AC-3 | A pair whose `prior` carries revision `b` and `superseding` revision `a` is compared in that direction: a case the `prior` unit admits and the `superseding` unit refuses is a regression. Swapping only the two revision labels leaves every case's two classes and its result unchanged. | Test (TC-860) |
| FR-340-AC-4 | A case stating no limit sets runs with each set's published default; the same case stating `accounting` with a work budget of one unit runs both revisions with that budget: both classes are `incomplete` and the case is `unresolved (incomplete)`. | Test (TC-860) |
| FR-340-AC-5 | Copying a corpus to another directory and running the gate there gives a report byte-equal to the original's. | Test (TC-860) |
| FR-340-AC-6 | `make refinement-versioning` runs the gate over `tests/fixtures/refinement/versioning/` and passes; adding the seeded-regression pair of TC-864 to that corpus makes the target fail. | Test (TC-864) |

## Dependencies

- FR-109 and FR-115 (`run_clause` and its selections), FR-106 (snapshot and
  invocation provisions), FR-056 and FR-099 (package and dependency input),
  FR-001 (source labels).
- FR-342, FR-343 and FR-344 (classification, comparison, report).
- QSpec FR-290 (the claim-form row a case tests).

## References

- ADR-017 §2 RF-1, RF-2 (amended 2026-10-01: corpus layout, pair
  direction), RF-5, RF-6.
- QSpec half: QSpec FR-452 and FR-290 (the claim-form row and the
  V1-TOOL-011 re-trace), STD-117.
- Implementation ticket QSL-40; specification ticket QSL-386.
