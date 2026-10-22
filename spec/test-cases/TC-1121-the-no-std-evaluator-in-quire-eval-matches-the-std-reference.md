---
id: TC-1121
title: "The no_std evaluator in quire-eval matches the std reference"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-403
    type: verifies
---
# TC-1121: The no_std evaluator in quire-eval matches the std reference

## Description

Verify FR-403: the `no_std` evaluator lives in `quire-eval` with FR-401's
dependency set, reads checked terms without building them, reads the
environment only through its contract, and returns the std reference's
results, refusal records and charges, including FR-262-AC-1 to AC-8. Scope:
FR-403-AC-1 to FR-403-AC-9.

## Test Procedure

1. In `agent-ix/quire-eval`, read `quire-eval/Cargo.toml`, run `cargo tree
   -p quire-eval -e features -i quire-checked`, and search `quire-eval/src`
   for `HashMap`, `HashSet` and `std::`.
2. In QSL, compile a fixture crate that names
   `qsl_semantics::value::divide` and one that names
   `qsl_semantics::value::modulo`. Over a package that declares an integer
   division law, evaluate `-7 div 2` and `-7 mod 2` through both
   evaluators with the `test-support` charge log.
3. Build FR-401-AC-5's fixture package. Create a registry with the `no_std`
   evaluator from its checked graph and one with the reference adapter, and
   make FR-401-AC-5's five registrations in a fresh registry each.
4. Run each FR-403-AC-4 vector through `qsl-eval`'s `quire-eval` entry and
   through `CheckedPackageEvaluation`, each with a fresh meter, and compare
   results, refusal records (same region resolver), consumed counters and
   admitted charges.
5. Build an `ObjectEnvironment` with one recorded population of three
   members and an attached `ReadLog`; evaluate an `allInstances` over it
   through both evaluators. Evaluate a state clause with a `pre(...)` field
   read over an admitted pre/post observation pair through both.
6. Evaluate an `allInstances` over an unrecorded `PopulationId` through the
   `no_std` evaluator with `quire-eval`'s closure-only environment, and
   through the reference over `ObjectEnvironment::new` of the same closure.
7. Run FR-262-AC-1 to FR-262-AC-8 through `qsl-eval`'s `quire-eval` entry,
   AC-1 and AC-2 on a thread with a 512 KiB stack.
8. Admit a population document whose three members are listed in two
   different orders, two of them violating a subsetting edge, ten times
   each.
9. Read `qsl-eval/Cargo.toml`, `qsl-eval`'s public exports and
   `quire-eval/src`.

## Expected Results

1. Only `quire-checked`, `quire-semantic-value`, `quire-exact`,
   `quire-walk` and `thiserror` are normal dependencies, with default
   features off; no `construct` feature appears; no match; no feature
   gates the evaluator.
2. Both fixture crates fail to compile; both evaluators return the same
   values and the same admitted charges.
3. Both give `Accepted`, `SignatureMismatch`, `Accepted`,
   `CallableNotAdmitted`, `CallableNotAdmitted`.
4. Every compared item is equal: `count(3)` completes with `3`; `d(0)` is
   located at the `div` node; `f(10)` is `CallFailure::Input` with no
   charge; the cancelled call is `CallFailure::Cancelled` with no charge;
   the `w - 1` run is `Incomplete` naming `work_units` at the same charge
   point with that charge unrecorded.
5. The members come back in the same order with the same charges, the
   `ReadLog` sets are equal, and the `pre(...)` read returns the same value
   and is reported with `Observation::Pre`.
6. Both refuse with the same code, cause, fields and location.
7. Each criterion passes with the reference's expected values; no depth
   refusal and no stack overflow occur.
8. Every run refuses naming the violating member with the lowest reference
   key.
9. `qsl-eval` declares no `std` feature, `call` and `evaluate` use the
   reference evaluator, no `quire-eval` item is re-exported, and
   `quire-eval/src` holds no name resolution, supplied-value admission,
   simulation or model admission.
