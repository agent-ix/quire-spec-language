---
id: FR-405
title: "Make quire-eval QSL's only evaluator and retire the std reference evaluator"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-262
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-401
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-402
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-403
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-107
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-106
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-138
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-139
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-265
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-366
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-380
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-391
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-031
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-295
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/NFR-016
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/AD-001
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: traces_to
---
# FR-405: Make quire-eval QSL's only evaluator and retire the std reference evaluator

## Description

[AD-001](../assurance/AD-001-shared-evaluation-core-interface.md) has two
implementations of the [FR-262](FR-262-evaluate-values-and-calls-at-any-depth.md)
semantics while the `no_std` evaluator is built: QSL's std reference
evaluator in `qsl-eval` (the `Machine` of
`qsl-eval/src/value/expression/evaluate.rs`), and the evaluator in
`quire-eval` ([FR-403](FR-403-implement-the-no-std-evaluator-in-quire-eval.md)).
[FR-402](FR-402-differentially-test-the-no-std-evaluator-against-the-std-reference.md)
keeps them equal. This requirement states the switch that ends that period
(slice S9): the evidence that allows it, the state the workspace is in after
it, and what it deletes.

After the switch, `quire-eval`'s evaluator is the only evaluator of checked
terms in QSL. The std reference evaluator is deleted. It is not kept behind
a feature, as a fallback or as a test oracle. QSL, RT and CG then share one
implementation of the evaluation semantics, which is the purpose of the
shared core.

## Inputs

- The switch base: the parent, on QSL `main`, of the switch change. Both
  implementations run on it.
- The `quire-eval` revision that QSL's `Cargo.lock` resolves.
- A bundle that CG's `make evaluator-corpus` writes from CG's `main`.

## Outputs

- The switch change: one QSL change that records the corpus expectations,
  routes every evaluation to `quire-eval` and deletes the std reference
  evaluator. Its description reports the switch base and the results of the
  runs made on it.
- After the switch, a regression harness that runs each corpus case once
  through `quire-eval` and compares the result with the case's recorded
  expectation.

## Behavior

### The switch criterion

The switch SHALL be made only when all of the following hold. Items 1 and 2
are measured on the switch base, items 3 to 5 at the switch change's head.

1. **Differential suite.** On the switch base, `make evaluator-differential`
   over the QSL-owned corpus (FR-402: the checked-in bundle and all 256
   generated cases) reports zero divergences. A run over a bundle that CG's
   `make evaluator-corpus` wrote from CG's `main`, through
   `make evaluator-differential EVALUATOR_CORPUS_DIR=<dir>`, also reports
   zero divergences (FR-402-AC-10).
2. **FR-262 under both implementations.** On the switch base,
   FR-262-AC-1 to AC-8, AC-10 and AC-11 pass with each implementation (FR-402-AC-9
   and FR-403-AC-7), AC-1 and AC-2 on a 512 KiB stack.
3. **The `no_std` gate.** FR-401-AC-1's `thumbv7em-none-eabi` build passes in
   `agent-ix/quire-eval` at the `quire-eval` revision QSL's `Cargo.lock`
   resolves at the switch head.
4. **The unsafe audit.** Slice S8's unsafe audit, NFR-016's
   `make audit-unsafe` gate, passes in `agent-ix/quire-eval` at that
   revision.
5. **Every evaluation family.** Every evaluation `qsl-eval` performs runs on
   `quire-eval`'s evaluator (FR-405-AC-4), and the tests FR-405-AC-5 names
   pass.

The switch deletes the std side, so items 1 and 2 cannot run at the switch
head. The switch change's description SHALL report the switch base and
the result of each run items 1 and 2 name; that report is their evidence.

RT's target acceptance of `quire-eval` and the deletion of RT's evaluator
port are not part of this criterion. They are RT's migration (FR-262-AC-9,
IR-349), which consumes the same `quire-eval` and compares it with the
recorded corpus expectations (AD-001 "RT and CG acceptance").

### Clause evaluation: a prerequisite

FR-403 limits `quire-eval`'s evaluator to the `Value` family. It faults on
`Pre`, `NamedPopulation`, `PopulationObjects` and `History`, and the clause
entries (FR-107 state clauses, FR-265 trails, the ADR-031 separating
witness, FR-366 value invariants, FR-106 check 6.5 provenance, FR-138
history, FR-139 refinement populations, FR-391 named populations) run on
the std reference evaluator. Retiring that evaluator therefore needs a
`quire-eval` entry for clause evaluation and a clause extension of its
environment contract. A requirement of its own defines that entry, the
extension's reads and reported events, and the placement of the clause and
refinement causes that FR-403 keeps in `qsl-eval` (`HistoryUnresolved`,
`NamedPopulationIncomplete`, `ModelQueryRefusal` and the `ProtocolClause*`
causes, one of which carries `qsl-semantics`' `AdmissionRecord`). Linear
ticket STD-318 tracks it.

This requirement places three constraints on that entry:

1. It is an entry of the one evaluator: it shares the node dispatch,
   scheduler, meter and charge points of the `Value`-family entry.
2. The `Value`-family Begin keeps FR-403's `outside-value-family` fault for
   the four node kinds, so RT's and CG's entry is unchanged.
3. `quire-eval` names no QSL type.

### One switch change

The switch SHALL be one change. The change that routes the production
evaluation paths to `quire-eval` also deletes the std reference evaluator.
The production evaluation paths are `CheckedPackageEvaluation::call` and
`evaluate`, `call_supplied`, `evaluate_closed`, the clause, clause-set,
frame, separation and value-invariant entries, and the simulation,
automaton, hyper, temporal, fairness and model-subject engines. FR-403's
separate `quire-eval` entry, which the differential runs use, is not a
production evaluation path. No commit of `main` holds both `Machine` and a
production evaluation path that evaluates on `quire-eval`. Before the
switch, the production evaluation paths run the std reference evaluator.

### The state after the switch

1. `quire-eval`'s evaluator is the only evaluator of checked terms in the
   QSL workspace. The production evaluation paths, `qsl-replay`,
   `qsl-analyze` and the interpreter that FR-295 compares the JIT and AOT
   backends with all reach evaluation only through `quire-eval`'s session.
   The root crate's `src/state/evaluation.rs` and `src/temporal.rs`
   evaluate artifact value graphs, not checked terms, are outside this
   requirement, and are deleted under ADR-011 M-6c.
2. These are deleted: the file `qsl-eval/src/value/expression/evaluate.rs`
   with its submodules `evaluate/invariants.rs` and `evaluate/union_case.rs`
   and the `Machine` type with its task and value stacks; the std reference
   adapter of FR-401, since `quire-eval` answers the closure contract itself
   (FR-403); and FR-403's separate `quire-eval` entry, since the production
   evaluation paths take its place. The FR-380-AC-7 population-bound check
   (`authored_bound_refusal` and `refused_evaluation`), which clause
   evaluation calls, moves into `qsl-eval`'s clause orchestration and is not
   deleted.
3. No Cargo feature, `cfg`, environment variable, command-line option or
   setting selects an evaluator. `quire-eval` has no feature that adds or
   removes its evaluator (FR-403), and no QSL manifest declares one. QSL's
   CLI, every backend and every test lane build the same evaluator with
   default features, with `--no-default-features` where a crate allows it,
   and with `--all-features`.
4. No `[dev-dependencies]` entry, `test-support` feature, `xtask` command,
   fixture crate or test module holds the std reference evaluator or a copy
   of it. The reference's results survive as recorded data (the corpus
   expectations and the expected values in the tests), not as code.
5. No item is left at a deleted path, and no deleted path re-exports or
   forwards to a `quire-eval` item (ADR-011 §7.2).

`qsl-eval` stays a `std` crate with no `std` feature. It keeps the clause
orchestration (clause sets, frames, separation, value invariants and the
simulation, automaton, hyper, temporal, fairness and model-subject engines)
as callers of `quire-eval`, the FR-380-AC-7 bound check, the environment
contract implementations over `ObjectEnvironment`, name resolution, the
FR-098 conversion phase and the refusal-record function.

`qsl-eval` holds no evaluator of checked terms: no type that keeps a task
stack or value stack of an evaluation, and no function that computes the
`Value`, `Outcome` or `Evaluation` of a checked node or checked expression
except by beginning a `quire-eval` session. A structural walk over checked
terms that computes no value is not evaluation and is allowed. ADR-031
SW-2's decision-path walk (`resolve_quantifier` in
`qsl-eval/src/value/expression/mod.rs`, and the same walk in
`qsl-replay`'s witness derivation) is such a walk: it matches node kinds to
find a quantifier's decision path and evaluates nothing.

### Tests at the switch

The switch change SHALL delete only the tests whose subject it deletes:
tests that run only the std reference evaluator as a second
implementation, tests that pair a std record with a `no_std` record, and
tests of FR-403's separate entry. Each such test whose criterion still holds
after the switch is ported to run once on `quire-eval`, keeping the
reference's values as its expected values. The tests inside the deleted
files, including the FR-090 and FR-096 probes in `evaluate/invariants.rs`,
are ported in the same way and not deleted. Every other test on the switch
base exists at the switch head with unchanged expected values.

### The corpus after the switch

The switch change SHALL record expectations for the QSL-owned corpus from
the switch base's zero-divergence run, in which each case's std and
`no_std` compared records are equal:

1. Each case of `tests/fixtures/evaluator-corpus/cases.jsonl` carries that
   record as its non-null `expected`.
2. The 256 generated cases are checked in as the bundle
   `tests/fixtures/evaluator-corpus/generated/cases.jsonl`, each with that
   record as its `expected`. The generator stays. Its output for the same
   seeds and count equals that bundle's records with each `expected`
   replaced by `null`.

After the switch the FR-402 harness runs one implementation. Its `xtask`
command and `make` target are named `evaluator-regression`; no `xtask`
command or `make` target named `evaluator-differential` exists, and none is
an alias of the other. The runner evaluates every admitted case once through
`quire-eval`, keeping QSL's admission through `qsl_replay::compile_package`
(FR-402), and compares the compared record with the case's `expected`:

- A record that differs from `expected` is a divergence, reported with the
  pair `expected/quire-eval` and the first differing JSON pointer with both
  values. `charges` are compared when neither is `null`.
- A case whose `expected` is `null` is a divergence reported as
  `missing_expected`. This holds for CG's bundle too: after the switch a CG
  case is checked only against the expectation CG records through
  `quire-eval` (FR-402 "What CG writes").
- A case for which the run produces no record is a divergence, as in
  FR-402.

The pass criterion stays zero divergences with no option that tolerates,
skips or excludes a case. `make ci` runs `evaluator-regression` over the
QSL-owned corpus. FR-262-AC-1 to AC-8 and FR-403's vectors run once, on
`quire-eval`, against the values the reference produced.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-405-AC-1 | On the switch base, `make evaluator-differential` over the QSL-owned corpus admits every checked-in case and all 256 generated cases and reports zero divergences, and `make evaluator-differential EVALUATOR_CORPUS_DIR=<dir>` over a bundle CG's `make evaluator-corpus` wrote from CG's `main` reports zero divergences; the switch change's description reports the switch base and both results. | Test (TC-1124) and Inspection |
| FR-405-AC-2 | On the switch base, FR-262-AC-1 to AC-8, AC-10 and AC-11 pass with each implementation, AC-1 and AC-2 on a 512 KiB stack with no depth refusal and no stack overflow; the switch change's description reports the result. | Test (TC-1124) and Inspection |
| FR-405-AC-3 | At the `quire-eval` revision QSL's `Cargo.lock` resolves at the switch head, FR-401-AC-1's `thumbv7em-none-eabi` build and NFR-016's `make audit-unsafe` gate both pass in `agent-ix/quire-eval`. | Test (TC-1124) |
| FR-405-AC-4 | At the switch head: `qsl-eval/src/value/expression/evaluate.rs` and its `evaluate/` submodules, the `Machine` type, the std reference adapter and FR-403's separate `quire-eval` entry do not exist; no type in `qsl-eval`'s shipped source keeps an evaluation's task or value stack; each `qsl-eval` function whose result is a `Value`, an `Outcome` or an `Evaluation` of a checked node or expression obtains it from a `quire-eval` session, and every `NodeKind` match left in `qsl-eval` is a structural walk that returns no value, such as the ADR-031 SW-2 decision-path walk; no QSL manifest and no `quire-eval` manifest declares a feature that selects an evaluator; no `[dev-dependencies]` entry, `test-support` feature, `xtask` command or fixture crate holds an evaluator of checked terms; and no deleted path is re-exported or forwards to `quire-eval`. | Inspection (TC-1124) |
| FR-405-AC-5 | A test inventory of the switch base and the switch head (every test function of the workspace, by crate, file and name, with its expected values) shows that every test absent at the head ran only the std reference as a second implementation, paired a std with a `no_std` record, or tested FR-403's separate entry, and that each test of the deleted files, including the `evaluate/invariants.rs` probes, exists ported at the head with its expected values. Every other base test exists at the head with unchanged expected values, and at the head the `qsl-eval`, `qsl-replay` and `qsl-analyze` suites, the FR-262 tests, the FR-096 refusal-record tests, TC-780's backend-versus-interpreter tests, the clause, separation (FR-265, ADR-031) and value-invariant tests, and the QSL conformance run over the QSpec checked-package fixtures pass. | Test (TC-1124) |
| FR-405-AC-6 | At the switch head, the `Value`-family Begin over `pre(allInstances<T>(p))` built by `check_postcondition_expression`, with `p: Population<T>[3]`, returns `CallFailure::Fault` with stage `quire-eval` and invariant `outside-value-family` and no charge. The clause entry over the same expression, with a pre observation recording `p`'s population as members `a`, `b` and a post observation recording `a`, `b`, `c`, completes with the collection of `a` and `b` in ascending reference-key order, with the admitted charges of the checked-in fixture that the switch base's reference produced for that expression. | Test (TC-1124) |
| FR-405-AC-7 | At the switch head, every case of the checked-in bundle and of `tests/fixtures/evaluator-corpus/generated/cases.jsonl` has a non-null `expected` equal to the switch base's std compared record for that case, and the generator run for count 256 yields that generated bundle's records with each `expected` replaced by `null`, byte for byte. | Test (TC-1124) |
| FR-405-AC-8 | At the switch head, `make ci` runs `evaluator-regression` and it reports zero divergences; no `xtask` command or `make` target named `evaluator-differential` exists. A bundle holding `fr402/count-3` whose `expected.consumed.work_units` is one less than the run's reports that case with pair `expected/quire-eval` and pointer `/consumed/work_units` and still runs every other case; the same case with `expected` `null` reports `missing_expected`. A comparator fed expectations and a `quire-eval` record set that differ only in one `charges` entry's `sizes` reports pair `expected/quire-eval` and a pointer under `/charges`, and a `quire-eval` record set that lacks one admitted case reports that case as a divergence naming `quire-eval` as producing no record. The command line, environment and configuration offer no setting that tolerates, skips or excludes a case. | Test (TC-1124) and Inspection |
| FR-405-AC-9 | At the switch head, `make evaluator-regression EVALUATOR_CORPUS_DIR=<dir>` over a bundle CG's `make evaluator-corpus` wrote, with every `expected` recorded through `quire-eval`, reports zero divergences, and the same bundle with one record's `expected` set to `null` reports that case as `missing_expected`. | Test (TC-1124) |
| FR-405-AC-10 | The change that routes the production evaluation paths to `quire-eval` is the change that deletes `Machine`: no commit of `main` holds both `Machine` and a production evaluation path that evaluates on `quire-eval`. FR-403's separate `quire-eval` entry is not a production evaluation path. | Inspection (TC-1124) |

## Dependencies

- [FR-262](FR-262-evaluate-values-and-calls-at-any-depth.md): the semantics
  and AC-1 to AC-8, which both implementations meet before the switch and
  `quire-eval` meets after it.
- [FR-401](FR-401-host-the-evaluation-interface-in-the-quire-eval-leaf.md):
  the leaf, its `thumbv7em-none-eabi` gate and the std reference adapter
  the switch deletes.
- [FR-402](FR-402-differentially-test-the-no-std-evaluator-against-the-std-reference.md):
  the differential suite whose result allows the switch, and the corpus,
  admission and compared record the regression harness keeps.
- [FR-403](FR-403-implement-the-no-std-evaluator-in-quire-eval.md): the
  `Value`-family evaluator, its environment contract and what `qsl-eval`
  keeps until the switch.
- The clause-evaluation requirement STD-318 tracks: the `quire-eval` clause
  entry and its environment contract.
- [FR-107](FR-107-evaluate-state-clauses-at-s6a.md),
  [FR-265](FR-265-derive-a-state-clause-separating-witness.md),
  [ADR-031](../decisions/ADR-031-state-forall-separating-witness.md),
  [FR-106](FR-106-admit-snapshots-and-invocations.md), FR-138, FR-139,
  FR-366, [FR-380](FR-380-bound-a-domain-package-population-and-link-a-reachability-to-its-node.md)
  and FR-391: the clause evaluations and checks the switch moves onto
  `quire-eval`.
- [FR-295](FR-295-return-interpreter-outcomes-from-jit-and-aot.md): the
  interpreter the backends are compared with.
- [NFR-016](../non-functional/NFR-016-audit-unsafe-code-in-the-evaluator-closure.md):
  slice S8's unsafe audit, `make audit-unsafe`.
- [AD-001](../assurance/AD-001-shared-evaluation-core-interface.md) slice S9
  (IR-593) and [ADR-011](../decisions/ADR-011-stage-dag-and-dependency-architecture.md)
  X-17.
