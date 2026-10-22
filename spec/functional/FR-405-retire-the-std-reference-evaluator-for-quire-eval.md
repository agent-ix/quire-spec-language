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
  - target: ix://agent-ix/quire-spec-language/FR-265
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-295
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
evaluator in `qsl-eval` (the `Machine` of `value::expression::evaluate`),
and the evaluator in `quire-eval`
([FR-403](FR-403-implement-the-no-std-evaluator-in-quire-eval.md)).
[FR-402](FR-402-differentially-test-the-no-std-evaluator-against-the-std-reference.md)
keeps them equal. This requirement ends that period (slice S9). It states
the evidence that allows the switch, the state the workspace is in after
it, and what the switch deletes.

After the switch, `quire-eval`'s evaluator is the only evaluator of checked
terms in QSL. The std reference evaluator is deleted. It is not kept behind
a feature, as a fallback, or as a test oracle. QSL, RT and CG then share one
implementation of the evaluation semantics, which is the purpose of the
shared core: one evaluator, with no duplicated module of any size to keep
equal by test.

## Inputs

- The switch base: the parent, on QSL `main`, of the switch change, on
  which both implementations still run.
- The `quire-eval` revision that QSL's `Cargo.lock` resolves.
- A bundle that CG's `make evaluator-corpus` writes from CG's `main`.

## Outputs

- The switch change: one QSL change that records the corpus expectations,
  routes every evaluation to `quire-eval` and deletes the std reference
  evaluator.
- After the switch, a regression harness that runs each corpus case once
  through `quire-eval` and compares it with its recorded expectation.

## Behavior

### The switch criterion

The switch SHALL be made only when all of the following hold. Items 1 and 2
are measured at the switch base; items 3 to 5 at the switch change's head.

1. **Differential suite.** At the switch base, `make evaluator-differential`
   over the QSL-owned corpus (FR-402: the checked-in bundle and all 256
   generated cases) reports zero divergences. A run over a bundle that CG's
   `make evaluator-corpus` wrote from CG's `main`, through
   `make evaluator-differential EVALUATOR_CORPUS_DIR=<dir>`, also reports
   zero divergences (FR-402-AC-10).
2. **FR-262 under both implementations.** At the switch base,
   FR-262-AC-1 to FR-262-AC-8 pass with each implementation (FR-402-AC-9
   and FR-403-AC-7), AC-1 and AC-2 on a 512 KiB stack.
3. **The `no_std` gate.** FR-401-AC-1's `thumbv7em-none-eabi` build passes in
   `agent-ix/quire-eval` at the `quire-eval` revision QSL's `Cargo.lock`
   resolves at the switch head.
4. **The unsafe audit.** The unsafe and dependency audit of slice S8
   (IR-592) passes for that revision of `quire-eval`, `quire-checked` and
   `quire-eval-corpus`.
5. **Every evaluation family.** Every evaluation `qsl-eval` performs runs on
   `quire-eval`'s evaluator (see "Clause evaluation"), and the suites named
   in FR-405-AC-5 pass with no edit to any expected value.

RT's target acceptance of `quire-eval` and the deletion of RT's evaluator
port are not part of this criterion. They are RT's migration (FR-262-AC-9,
IR-349), which consumes the same `quire-eval` and needs no QSL evaluator.

### One switch change

The switch SHALL be one change: the change that routes
`CheckedPackageEvaluation::call` and `evaluate` to `quire-eval` also routes
every clause entry to it and deletes the std reference evaluator. No commit
of `main` has some evaluation on the std reference evaluator while another
runs on `quire-eval` as its production path. Before the switch, `call` and
`evaluate` run the std reference evaluator, as FR-403-AC-9 states.

### The state after the switch

1. `quire-eval`'s evaluator is the only code in the QSL workspace that
   computes the value of a checked term. `CheckedPackageEvaluation::call`
   and `evaluate`, `call_supplied`, `evaluate_closed`, the clause,
   clause-set, frame, separation and value-invariant entries, the
   simulation, automaton, hyper, temporal, fairness and model-subject
   engines, `qsl-replay`, `qsl-analyze`, and the interpreter that FR-295
   compares the JIT and AOT backends with, all drive `quire-eval`'s session.
2. These are deleted: the `Machine` type and the `value::expression::evaluate`
   module with its task and value stacks; the std reference adapter of
   FR-401, since `quire-eval` answers the closure contract itself (FR-403);
   and `qsl-eval`'s separate `quire-eval` entry of FR-403, since `call` and
   `evaluate` are that entry.
3. No Cargo feature, `cfg`, environment variable, command-line option or
   setting selects an evaluator. `quire-eval` has no feature that adds or
   removes its evaluator (FR-403), and no QSL manifest declares one. QSL's
   CLI, every backend and every test lane build the same evaluator with
   default features, with `--no-default-features` where a crate allows it,
   and with `--all-features`.
4. No `[dev-dependencies]` entry, `test-support` feature, `xtask` command,
   fixture crate or test module holds the std reference evaluator or a copy
   of it. The reference's results survive as recorded data (the corpus
   expectations below and the expected values in the tests), not as code.
5. No item is left at a deleted path, and no deleted path re-exports or
   forwards to a `quire-eval` item (ADR-011 §7.2).

`qsl-eval` stays a `std` crate with no `std` feature. It keeps the clause
orchestration (clause sets, frames, separation, value invariants and the
simulation, automaton, hyper, temporal, fairness and model-subject engines)
as callers of `quire-eval`, the environment contract implementations over
`ObjectEnvironment`, name resolution, the FR-098 conversion phase, the
refusal-record function, and the clause and refinement causes FR-403 leaves
in it. Its shipped source names no `quire_checked::NodeKind` variant: the
dispatch on a checked node's kind is `quire-eval`'s.

### Clause evaluation

FR-403 limits `quire-eval`'s evaluator to the `Value` family. It faults on
`Pre`, `NamedPopulation`, `PopulationObjects` and `History`, and clause
evaluation runs on the std reference evaluator. Retiring that evaluator
therefore requires `quire-eval` to evaluate every checked input a clause
entry evaluates. Before the switch, `quire-eval` SHALL:

1. Evaluate the four node kinds, and the anchor switch `pre(...)` makes,
   through an entry for clause evaluation that takes an environment
   implementing a clause extension of its environment contract. The
   extension adds the reads those kinds make: the pre and current
   observations and the per-read observation FR-107 assigns, history fields
   (FR-138), refinement population objects (FR-139) and named populations
   (FR-391).
2. Report, through that contract, the events from which `qsl-eval` builds
   FR-265's trail, the witnessed-read provenance of FR-106 check 6.5, the
   ADR-031 separating witness and the stop report. `quire-eval` names none
   of `qsl_foundation::witness`'s types or any other QSL type, so that
   vocabulary stays in QSL.
3. Raise the protocol-clause refusals that clause evaluation raises today
   with the same catalog codes, fields and locations.

The `Value`-family Begin keeps FR-403's `outside-value-family` fault for
the four node kinds, so RT's and CG's entry is unchanged. The clause entry
is a separate entry of the same evaluator: it shares the node dispatch,
scheduler, meter and charge points of the `Value`-family entry.

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

After the switch, the FR-402 harness runs one implementation. The `xtask`
command and the `make` target are `evaluator-regression`; the name
`evaluator-differential` no longer exists and has no alias. The runner
evaluates every admitted case once through `quire-eval`, keeping QSL's
admission through `qsl_replay::compile_package` (FR-402), and compares the
compared record with the case's `expected`:

- A record that differs from `expected` is a divergence, reported with the
  pair `expected/quire-eval` and the first differing JSON pointer with both
  values. `charges` are compared when neither is `null`.
- A case whose `expected` is `null` is a divergence reported as
  `missing_expected`. This applies to CG's bundle too: after the switch a
  CG case is checked only through the expectation CG records through
  `quire-eval` (FR-402 "What CG writes").
- A case for which the run produces no record is a divergence, as in
  FR-402.

The pass criterion stays zero divergences with no option that tolerates,
skips or excludes a case. `make ci` runs `evaluator-regression` over the
QSL-owned corpus. The `std/no_std` pair, the std run, the two-record
pairing, and the runs of FR-262's criteria against two implementations are
deleted with the reference. FR-262-AC-1 to AC-8 and FR-403's vectors then
run once, on `quire-eval`, against the values the reference produced.

### The umbrella's completion

The dual-implementation period (IR-584) ends when the switch change is on
`main` and FR-405-AC-1 to FR-405-AC-10 pass. The slices before it are
complete when their own criteria pass: FR-400, FR-401, NFR-015, FR-403,
FR-402 and slice S8's audit.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-405-AC-1 | At the switch base, `make evaluator-differential` over the QSL-owned corpus admits every checked-in and all 256 generated cases and reports zero divergences, and `make evaluator-differential EVALUATOR_CORPUS_DIR=<dir>` over a bundle CG's `make evaluator-corpus` wrote from CG's `main` reports zero divergences. | Test (TC-1124) |
| FR-405-AC-2 | At the switch base, FR-262-AC-1 to FR-262-AC-8 pass with each implementation, AC-1 and AC-2 on a 512 KiB stack with no depth refusal and no stack overflow. | Test (TC-1124) |
| FR-405-AC-3 | At the `quire-eval` revision QSL's `Cargo.lock` resolves at the switch head, FR-401-AC-1's `thumbv7em-none-eabi` build and slice S8's unsafe audit both pass in `agent-ix/quire-eval`. | Test (TC-1124) |
| FR-405-AC-4 | At the switch head: `Machine`, the module `qsl_eval::value::expression::evaluate`, the std reference adapter and `qsl-eval`'s separate `quire-eval` entry do not exist; `qsl-eval`'s shipped source names no `NodeKind` variant; no QSL manifest and no `quire-eval` manifest declares a feature that selects an evaluator; no `[dev-dependencies]` entry, `test-support` feature, `xtask` command or fixture crate holds an evaluator of checked terms; and no deleted path is re-exported or forwards to `quire-eval`. | Inspection (TC-1124) |
| FR-405-AC-5 | At the switch head, with no edit to any expected value relative to the switch base, the `qsl-eval`, `qsl-replay` and `qsl-analyze` suites, the FR-262 tests, the FR-096 refusal-record tests, TC-780's backend-versus-interpreter tests, the clause, separation (FR-265, ADR-031) and value-invariant tests, and the QSL conformance run over the QSpec checked-package fixtures pass. | Test (TC-1124) |
| FR-405-AC-6 | At the switch head, the `Value`-family Begin over `pre(allInstances<T>(p))` built by `check_postcondition_expression`, with `p: Population<T>[3]`, returns `CallFailure::Fault` with stage `quire-eval` and invariant `outside-value-family` and no charge. The clause entry over the same expression, with a pre observation recording `p`'s population as members `a`, `b` and a post observation recording `a`, `b`, `c`, completes with the collection of `a` and `b` in ascending reference-key order, with the admitted charges the switch base's reference recorded for it. | Test (TC-1124) |
| FR-405-AC-7 | At the switch head, every case of the checked-in bundle and of `tests/fixtures/evaluator-corpus/generated/cases.jsonl` has a non-null `expected` equal to the switch base's std compared record for that case, and the generator run for count 256 yields that generated bundle's records with each `expected` replaced by `null`, byte for byte. | Test (TC-1124) |
| FR-405-AC-8 | At the switch head, `make ci` runs `evaluator-regression` and it reports zero divergences; `evaluator-differential` exists neither as an `xtask` command nor as a `make` target. A bundle holding `fr402/count-3` whose `expected.consumed.work_units` is one less than the run's reports that case with pair `expected/quire-eval` and pointer `/consumed/work_units` and still runs every other case; the same case with `expected` `null` reports `missing_expected`. The command line, environment and configuration offer no setting that tolerates, skips or excludes a case. | Test (TC-1124) and Inspection |
| FR-405-AC-9 | At the switch head, `make evaluator-regression EVALUATOR_CORPUS_DIR=<dir>` over a bundle CG's `make evaluator-corpus` wrote, with every `expected` recorded through `quire-eval`, reports zero divergences, and the same bundle with one record's `expected` set to `null` reports that case as `missing_expected`. | Test (TC-1124) |
| FR-405-AC-10 | The change that routes `CheckedPackageEvaluation::call` and `evaluate` to `quire-eval` is the change that deletes `Machine`: no commit of `main` holds both `Machine` and a production path that evaluates on `quire-eval`. | Inspection (TC-1124) |

## Dependencies

- [FR-262](FR-262-evaluate-values-and-calls-at-any-depth.md): the semantics
  and AC-1 to AC-8, which both implementations meet before the switch and
  `quire-eval` meets after it.
- [FR-401](FR-401-host-the-evaluation-interface-in-the-quire-eval-leaf.md):
  the leaf, its `thumbv7em-none-eabi` gate and the std reference adapter
  this switch deletes.
- [FR-402](FR-402-differentially-test-the-no-std-evaluator-against-the-std-reference.md):
  the differential suite whose result allows the switch, and the corpus,
  admission and compared record the regression harness keeps.
- [FR-403](FR-403-implement-the-no-std-evaluator-in-quire-eval.md): the
  `Value`-family evaluator, its environment contract and what `qsl-eval`
  keeps until the switch.
- [FR-107](FR-107-evaluate-state-clauses-at-s6a.md),
  [FR-265](FR-265-derive-a-state-clause-separating-witness.md) and
  [ADR-031](../decisions/ADR-031-state-forall-separating-witness.md): the
  clause observations, trail and separating witness that clause evaluation
  produces.
- [FR-295](FR-295-return-interpreter-outcomes-from-jit-and-aot.md): the
  interpreter the backends are compared with.
- Slice S8 (IR-592): the unsafe and dependency audit.
- [AD-001](../assurance/AD-001-shared-evaluation-core-interface.md) slice S9
  (IR-593) and [ADR-011](../decisions/ADR-011-stage-dag-and-dependency-architecture.md)
  X-17.
