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

Verify FR-403. The `no_std` evaluator lives in `quire-eval` with FR-401's
dependency set and reads checked terms without building them. It admits
arguments itself and reads the environment only through its contract. It
returns the std reference's results, refusal records, charges and orders,
including FR-262-AC-1 to AC-8, AC-10 and AC-11. Each moved item has one definition. Scope:
FR-403-AC-1 to FR-403-AC-14.

## Test Procedure

1. In `agent-ix/quire-eval`, read `quire-eval/Cargo.toml` and run
   `cargo tree -p quire-eval -e features -i quire-checked`.
2. In QSL, compile one fixture crate for each of these paths:
   `qsl_semantics::value::divide`, `qsl_semantics::value::modulo`,
   `qsl_semantics::model::refusal::render_identity` and
   `qsl_semantics::value::supplied::SuppliedRun`. Name each item at its
   new path. Over a package that declares an integer division law,
   evaluate `-7 div 2` and `-7 mod 2` through both evaluators with the
   `test-support` charge log. Call `quire_exact::divide` and
   `quire_exact::modulo` directly with that law's profile and a fresh meter.
3. Build FR-401-AC-5's fixture package. Create one registry with the
   `no_std` evaluator from the package's checked graph and one with the
   reference adapter. Make FR-401-AC-5's five registrations, each in a
   fresh registry.
4. Run each FR-403-AC-4 vector through `qsl-eval`'s `quire-eval` entry and
   through `CheckedPackageEvaluation`, each with a fresh meter. Compare the
   results, the refusal records (with the same region resolver), the
   consumed counters and the admitted charges.
5. Build an `ObjectEnvironment` with one recorded population of three
   members and an attached `ReadLog`. Call a function whose body is
   `allInstances` over its `Population` parameter through both evaluators.
6. Over `quire-eval`'s closure-only environment, call a function with a
   `Population` parameter, passing an unrecorded `PopulationId`. Make the
   same call on the reference over `ObjectEnvironment::new` of the same
   closure.
7. Run FR-262-AC-1 to FR-262-AC-8, AC-10 and AC-11 (including AC-11's
   unresolved, mismatched-maximum and resolved `Population` child requests) through `qsl-eval`'s `quire-eval` entry,
   with AC-1 and AC-2 on a thread with a 512 KiB stack.
8. Admit a population document ten times in each of two member orders. It
   has three members, two of which violate a subsetting edge.
9. Read `qsl-eval/Cargo.toml`, `qsl-eval`'s public exports and source, and
   `quire-eval/src`.
10. Ten times each, through both evaluators: run step 5 over a population
    recorded with its members in descending reference-key order, and run
    FR-262-AC-8's kernel-loss case.
11. Through each evaluator, run a `lookup<T>(p, r) absent undefined` whose
    `r` is absent from the population. Read the undefined result's catalog
    code and fields, and read `FamilyKind::StateModel.catalog_code_prefix()`.
12. Through each evaluator with the charge log: evaluate the checked
    expression `x + 1` (`x: Int[0, 9]`) with `10` and with `3`; run a chain
    of three calls at `work_units` `3` and at `2`.
13. Build `pre(allInstances<T>(p))` with `p: Population<T>[3]` through
    `check_postcondition_expression` and begin it on the `no_std`
    evaluator. Build `allInstances<T>(p)` through `check_expression` and
    again through `check_postcondition_expression`. Evaluate each through
    both evaluators over an `ObjectEnvironment` that records `p`'s
    population with three members.
14. Call `function refers(): Boolean pure { protocol Fill }` (the unit
    declares `Fill`) through both evaluators.

## Expected Results

1. The normal dependencies are only `quire-checked`, `quire-semantic-value`,
   `quire-exact`, `quire-walk` and `thiserror`, each with default features
   off. No `construct` feature appears, and no feature gates the evaluator.
2. Each fixture crate fails to compile, and each new path resolves. Both
   evaluators return the values and admitted charges of the direct
   `quire_exact` calls.
3. Both registries give `Accepted`, `SignatureMismatch`, `Accepted`,
   `CallableNotAdmitted`, `CallableNotAdmitted`.
4. Every compared item is equal:
   - `count(3)` completes with `3`.
   - `d(0)` is located at the `div` node.
   - `f(10)` and the two-argument call of `f` are `CallFailure::Input`,
     the second one `Arity`, each with no charge.
   - `f(3)` over a meter whose installed handle is already cancelled is
     the located `Incomplete` at its `function.call` charge, naming
     `work_units` with `limit` equal to `consumed`, with no admitted
     charge, and the handle's `tripped()` returns its cause.
   - `f(3)` with a supplied-value run whose handle is already cancelled is
     `CallFailure::Cancelled` from argument admission, with no charge.
   - The `w - 1` run is `Incomplete` naming `work_units` at the same charge
     point, with that charge unrecorded.
5. The members come back in the same order with the same charges, and the
   two `ReadLog` sets are equal.
6. Both return `CallFailure::Input(InputRefusal::WrongValueKind)` naming
   parameter `0`, with no charge.
7. Each criterion passes with the reference's expected values. AC-11's unresolved `Population` child request and its request resolving to declared maximum `4` each end with `CallFailure::Fault` (`quire-eval`, `child-request-not-admitted`) and no child charge, and the resolved one is applied with one `function.call` charge. No depth
   refusal and no stack overflow occur.
8. Every run refuses, naming the violating member with the lowest reference
   key.
9. `qsl-eval` declares no `std` feature and defines no argument admission,
   `call` and `evaluate` use the reference evaluator, and no `quire-eval`
   item is re-exported. `quire-eval/src` holds no name resolution, FR-098
   conversion, model admission, simulation, frame check, clause-set
   combination, separation-check orchestration or value-invariant
   collection, and no entry besides the `Value`-family entry and FR-406's
   clause entry.
10. On every run the members come back ascending and equal across the two
    evaluators, and the loss lists are equal and in the same order.
11. Both results carry the prefix `state-model` and the `binding` and `key`
    fields that `render_identity` gives. The prefix is SV's constant, and
    `qsl-eval`'s source defines none of the moved cause types.
12. `10` is `CallFailure::Input` with no charge; `3` completes with `4` and
    no `function.call`. At `3` the chain completes with three `function.call`
    charges; at `2` it is `Incomplete` naming `work_units` at the third
    `function.call`, with two calls in the counter and the log. Both
    evaluators agree on every item.
13. The first is `CallFailure::Fault` with stage `quire-eval` and invariant
    `outside-value-family`, with no charge. Each of the other two gives
    equal results from both evaluators.
14. Both return `FamilyResult::Refused` with
    `unknown_required_feature`/`unsupported-feature`, `construct` =
    `protocol`, located at the reference, with equal records and charges.
