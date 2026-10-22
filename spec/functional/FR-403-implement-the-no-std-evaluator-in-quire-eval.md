---
id: FR-403
title: "Implement the no_std evaluator in quire-eval over checked terms and an environment contract"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-262
    type: implements
  - target: ix://agent-ix/quire-spec-language/FR-401
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-400
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-096
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/AD-001
    type: traces_to
---
# FR-403: Implement the no_std evaluator in quire-eval over checked terms and an environment contract

## Description

[FR-262](FR-262-evaluate-values-and-calls-at-any-depth.md) requires an
implementation of the shared evaluation interface that runs with `core` and
`alloc` only, so that RT and CG can evaluate checked code on a `no_std`
target. [FR-401](FR-401-host-the-evaluation-interface-in-the-quire-eval-leaf.md)
places that interface in the leaf `quire-eval` (ADR-011 layer E), which is
`#![no_std]`, has no `std` feature and names no QSL item, and leaves to this
requirement which checked terms the implementation reads.
[FR-400](FR-400-separate-checked-terms-from-the-checker.md) puts those
checked terms in `quire-checked` (layer C).

The crate `quire-eval` SHALL hold the `no_std` evaluator. It evaluates
checked expressions and applications over C's checked terms, reads the
object environment only through an environment contract that `quire-eval`
defines, and is driven by the FR-262 scheduler that `quire-eval` already
holds. `qsl-eval` consumes it: `qsl-eval` keeps the std reference
evaluator, the std reference adapter and an implementation of the
environment contract over `qsl-semantics`' `ObjectEnvironment`, and stays a
`std` crate. The two implementations agree on every result, refusal,
location, loss and charge, which FR-402's differential suite checks while
both exist.

This requirement also states which `qsl-semantics` code the evaluator needs
(slice S4, IR-588) and where each part goes: code that needs only the meter
and kernel, SV and C values moves into `quire-eval`; the model reads over an
admitted environment stay in `qsl-semantics` and reach the leaf only through
the environment contract.

## Inputs

- A checked closure: a `quire_checked::CheckedGraph`, whose scope view holds
  each imported library's checked graph keyed by its `PackageId` (FR-400).
- A checked input: a callable `NodeKey` with completed arguments, or a
  `quire_checked::CheckedExpression` with completed parameter values.
- An environment: a value of a type that implements the environment
  contract.
- One `quire_exact::Meter` and one `quire_exact::Cancel`.
- For a registered generated body, the FR-262 body factory.

## Outputs

- A terminal `quire_eval::Evaluation` or `quire_eval::CallFailure` from the
  FR-262 session, with the meter's spend.
- Reads reported to the environment through the contract.

## Behavior

### Where the implementation lives

1. The `no_std` evaluator SHALL be part of the crate `quire-eval` in the
   repository `agent-ix/quire-eval`. It SHALL be built in every build of
   that crate: `quire-eval` has no feature that adds or removes it.
2. `quire-eval`'s `[dependencies]` SHALL stay within FR-401's set
   (`quire-checked`, `quire-semantic-value`, `quire-exact`, `quire-walk`
   and `thiserror`, each without `std`). It SHALL take `quire-checked`
   without its `construct` feature: the evaluator reads checked terms and
   never builds one (ADR-011 FB-14). It SHALL name no `HashMap` or
   `HashSet` and no `std::` path, and a collection whose iteration order
   reaches an evaluation, a refusal, a loss list, a charge or a reported
   read SHALL be ordered (`BTreeMap`, `BTreeSet`, or a `Vec` in a stated
   order).
3. The evaluator SHALL be one implementation of FR-401's closure contract:
   the callable identity, the admitted signature, the checked input and the
   interpreted transitions of FR-401 items 1 to 4 are answered by
   `quire-eval` itself over C's checked terms, so a registry is created
   directly from a checked closure with no QSL adapter in between.

### The checked closure and checked input it reads

1. A registry SHALL be created from exactly one checked closure, a
   `CheckedGraph`, which it holds for its lifetime (FR-401 item 5).
2. The admitted callables of that closure are the functions of the graph
   and of each imported graph its scope view holds that are callable by
   name, each identified by its `CallableFunction` `NodeKey`, with the
   parameter and result `ValueType`s that `CallableFunction` records. An
   FR-151 synthesized dispatch candidate body or precondition is not an
   admitted callable. Registration decides `CallableNotAdmitted` and
   `SignatureMismatch` from this set alone, as FR-401 states for the
   reference adapter, so both give the same admission result for every
   registration.
3. A cross-package call SHALL resolve its callee in the imported graph the
   scope view keys by the callee's `PackageId`; the evaluator resolves no
   name.
4. The implementation SHALL read checked terms only through
   `quire-checked`'s public read accessors.

### The environment contract

`quire-eval` SHALL define an environment contract: the reads an evaluation
makes of the objects and populations it runs over. Every read names the
`Observation` (`Current`, `Pre` or `Post`, a C value) it reads, so one
environment value serves an invocation's pre-state and post-state. The
contract's answers are C, SV and K values or boxed `CatalogCoded` /
`UndefinedCoded` causes (SV); it names no QSL type. The reads are those the
std reference evaluator makes of `ObjectEnvironment`:

1. the SV `ObjectClosure` of an observation;
2. whether references in the observation are unresolved;
3. whether a field read of an object is witnessed out of its declared range
   (FR-106 check 6.5);
4. a named population's completeness and its members, ascending by object
   reference;
5. the FR-364 relationship extents: whether a relationship's extent is
   complete, its instances, and the manifest a `reaches` query resolves
   through;
6. an `allInstances` query and a `lookup` query over a `Population` value,
   each answered under the session meter, returning the value, a stop, or a
   refusal or undefined cause, with the population's members in ascending
   reference-key order;
7. a report of each field read and population-membership read the
   evaluation makes, with the observation, object, owning effective type and
   attribute name, or the population's declaration key and the one object
   looked up.

The contract SHALL NOT expose a way to change the environment. Population
and model admission (`admit_binding`, `admit_invocation`, the observation
admission and model normalization) happen before Begin, outside the
contract.

`quire-eval` SHALL provide one implementation of the contract over an SV
`ObjectClosure` alone, with no population recorded, no relationship extents,
no witnessed range and no read report. For every read it SHALL give the
answer the std reference gives for `ObjectEnvironment::new` over the same
closure: an `allInstances` or `lookup` over a `Population` value refuses
exactly as the reference refuses an unrecorded `PopulationId` (FR-089-AC-4).

`qsl-eval` SHALL implement the contract over `qsl-semantics`'
`ObjectEnvironment` (with its `AdmittedObservations` for clause and frame
evaluation). Item 6 SHALL call `qsl-semantics`' `value::model_query`
(`evaluate_lookup`, `evaluate_all_instances`) with the session meter, so its
charges and their order are the reference's. Its causes (a `ModelRefusal`,
an absent key, an operator-ineligible query) reach the leaf as the same
boxed catalog causes the reference evaluator raises. Item 7 records an
`ObservedRead` in the environment's `ReadLog` when one is attached.

### Which `qsl-semantics` code the evaluator needs (S4)

The evaluator-needed `qsl-semantics` code is the set of shipped
`qsl-semantics` items the reference evaluator's shipped modules name once
FR-400's checked terms are in C. Each part SHALL have exactly one of these
dispositions:

| Code | Disposition |
| --- | --- |
| The checked terms, `DeclarationKey`, `Observation` and the other values FR-400 lists | In C (FR-400); no S4 change |
| `value::definition::divide` and `value::definition::modulo`, which need only an `AdmittedIntegerDivision` (C), K's integers and the meter | Move to `quire-eval`. Both the reference evaluator and the `no_std` evaluator call them there, and no `qsl_semantics::` path to them remains |
| `model::object_environment` (`ObjectEnvironment`, `ObservedRead`, `ReadLog`), `model::population` (`PopulationBinding` and its queries), `value::model_query`, `model::observation`'s admitted-observation and frame types, `model::normalize`'s `ModelRefusal` and `ModelRefusalCause`, `model::refusal` | Stay in `qsl-semantics` with `std`. The leaf reaches them only through the environment contract that `qsl-eval` implements. `PopulationBinding` holds a domain package and a `ModelIndex`, which are not leaf vocabulary (FR-401) |
| `value::supplied` (`SuppliedRun`, `SuppliedLimits` and the supplied-value admission) | Stays in `qsl-semantics`. Supplied-value admission runs in `qsl-eval` before Begin, and the leaf never runs it |

`qsl-semantics` gains no `std` feature from this slice. No evaluator-needed
part stays in `qsl-semantics` that the `no_std` evaluator calls directly.

The `HashMap` in `model::population`'s subsetting check (`member_by_object`,
built during `admit_binding`) is read by key only. The admission walks
`admitted` in ascending key order, so the map's order reaches no refusal, and
admission refuses the same member for the same document on every run. It is
admission code, which stays `std`.

### Results, refusals and accounting

For every checked input that both implementations accept, the `no_std`
evaluator SHALL return the reference's results:

1. **Evaluation.** The same `FamilyOutcome<Value>` arm and value, the same
   location and the same loss list in the same order. A family refusal or
   undefined result SHALL have the same catalog code, cause tag, catalog
   fields and category. Each implementation raises its own cause values,
   which are compared through `CatalogCoded` and `UndefinedCoded`.
2. **Call failure.** The same `CallFailure` arm: `Input` with an equal
   `InputRefusal`, `Cancelled` with an equal `CancelCause`, `Storage` with
   the same parameter and failure. `Fault` is compared by arm and category.
   `Supplied` and `UnsupportedHelper` arise only from the supplied-value
   admission, which runs before Begin.
3. **Refusal record.** `qsl-eval`'s refusal-record function (FR-400 Source
   locations) over the two evaluations and the same region resolver SHALL
   return equal FR-096 records.
4. **Accounting.** The same consumed counters, the same ordered sequence of
   admitted charges (`Meter::admitted_charges` under `quire-exact`'s
   `test-support` feature), and, for a denied charge, the same
   `Incomplete` at the same charge point with the denied charge unrecorded.
   Cancellation is polled at every charge and before each scheduler
   transition, as FR-262 states.
5. **Depth.** No depth setting, depth refusal or native recursion whose
   depth grows with the input (FR-262 Behavior 1, ADR-030).

### What `qsl-eval` keeps

`qsl-eval` SHALL stay a `std` crate with no `std` feature of its own and no
NFR-015 row. It keeps:

- the std reference evaluator (`value::expression`'s `Machine` and its
  causes), which `CheckedPackageEvaluation`'s `call`, `evaluate` and the
  clause, frame, separation, clause-set and value-invariant entries use by
  default while it remains the reference;
- the std reference adapter of FR-401, and the environment contract
  implementation over `ObjectEnvironment`;
- name resolution (`QualifiedName`), argument admission for supplied values
  (`call_supplied`), the S6a family seam over the `qsl-semantics` family
  marker types, and the refusal-record function;
- the simulation, automaton, hyper, temporal, fairness and model-subject
  engines, which evaluate their checked expressions through the same
  default as `call` and `evaluate`.

`qsl-eval` SHALL expose an entry that runs a checked package's call or
expression through the `quire-eval` evaluator, with the package's checked
graph as the closure and its `ObjectEnvironment` as the environment. The
FR-402 differential runs and this requirement's FR-262 runs use that entry.
`qsl-eval` re-exports no `quire-eval` item.

In QSL, the `no_std` evaluator's checked input is the admitted
`CheckedPackage`'s checked graph, handed over in the process that admitted
it. No byte form carries an evaluable checked graph: the
`quire.checked-package/v2` wire's I2 reader yields an import view, not a
checked graph. Because `quire-eval` has no `std` feature, a build that also
links QSL's `std` crates compiles the same evaluator source as
`quire-eval`'s `thumbv7em-none-eabi` build.

The host-body registry, its admission result, the session, its step, the
body contract and the child request stay in `quire-eval` (FR-401), so
FR-262-AC-3, AC-4 and AC-6 to AC-8 exercise the same scheduler under both
implementations.

### FR-262 with `std` off

`quire-eval` has no `std` feature, so a host test build compiles its
evaluator from the same source as its `thumbv7em-none-eabi` build. In QSL,
FR-262-AC-1 to FR-262-AC-8 SHALL each also run with the `no_std` evaluator
as the implementation: QSL's checker admits the source and `qsl-package`
links the checked package with `std`, then the `quire-eval` evaluator
evaluates its checked graph. Each run SHALL meet that criterion's
expectations with the reference's values, and FR-262-AC-1 and AC-2 run on a
thread with a 512 KiB stack. That `quire-eval` builds without `std` is
FR-401-AC-1, checked in its own repository's gate.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-403-AC-1 | `quire-eval`'s manifest names only `quire-checked`, `quire-semantic-value`, `quire-exact`, `quire-walk` and `thiserror` as normal dependencies, with default features off, and `cargo tree -p quire-eval -e features -i quire-checked` in `agent-ix/quire-eval`'s gate shows no `construct` feature. Its source names no `HashMap`, `HashSet` or `std::` path. It declares no feature that adds or removes the evaluator. | Test (TC-1121) |
| FR-403-AC-2 | `qsl_semantics::value::divide` and `qsl_semantics::value::modulo` fail to compile from another crate, and the two functions resolve in `quire-eval`. With the admitted integer division law of a package that declares one, `-7 div 2` and `-7 mod 2` return the same values and the same admitted charges through the reference evaluator and the `no_std` evaluator as before the move. | Test (TC-1121) |
| FR-403-AC-3 | Over FR-401-AC-5's fixture (a package with `f(x: Int[0, 9]): Boolean`, an FR-151 dispatched member call, and a dependency with `g(): Integer`), a registry the `no_std` evaluator creates from the package's checked graph returns `Accepted`, `SignatureMismatch`, `Accepted`, `CallableNotAdmitted` and `CallableNotAdmitted` for the five registrations FR-401-AC-5 lists, equal to the reference adapter's results. | Test (TC-1121) |
| FR-403-AC-4 | For each vector, the `no_std` evaluator and the reference return equal results, equal FR-096 refusal records, equal consumed counters and equal ordered admitted charges: `count(3)` completes with `3`, with four `function.call` charges among its admitted charges; `function d(x: Int[0, 9]): Integer pure { 10 div x }` called with `0` returns the reference's non-completed outcome located at the `div` node; `f` called with `10` returns `CallFailure::Input` with no charge; a call begun with an already-cancelled `Cancel` returns `CallFailure::Cancelled` with no charge; and `count(3)` with `work_units` one below its measured spend returns `Incomplete` naming `work_units` at the reference's denied charge point, with that charge unrecorded and the earlier spend kept. | Test (TC-1121) |
| FR-403-AC-5 | Over an `ObjectEnvironment` with one population of three recorded members, an `allInstances` over that population evaluated through the `no_std` evaluator with `qsl-eval`'s environment implementation returns the members in the reference's order, with the reference's charges, and leaves the same `ObservedRead` set in an attached `ReadLog`. A `pre(...)` field read in a state clause evaluated over an admitted pre/post observation pair returns the reference's value and reports its read with `Observation::Pre`. | Test (TC-1121) |
| FR-403-AC-6 | Over `quire-eval`'s closure-only environment, an `allInstances` over a `Population` value whose `PopulationId` is not recorded returns the same refusal (code, cause, fields and location) as the reference evaluator over `ObjectEnvironment::new` of the same closure. | Test (TC-1121) |
| FR-403-AC-7 | FR-262-AC-1 to FR-262-AC-8 each pass a second time with the `no_std` evaluator as the implementation, through `qsl-eval`'s `quire-eval` entry, with the reference's expected values; the AC-1 and AC-2 runs use a thread with a 512 KiB stack and observe no depth refusal and no stack overflow. | Test (TC-1121) |
| FR-403-AC-8 | Admitting a population document whose three members are listed in two different orders, with a subsetting violation on two of them, refuses both times naming the violating member with the lowest reference key, on each of ten runs. | Test (TC-1121) |
| FR-403-AC-9 | `qsl-eval`'s manifest declares no `std` feature; `CheckedPackageEvaluation::call` and `evaluate` run the reference evaluator; `qsl-eval` re-exports no `quire-eval` item; and `quire-eval`'s source contains no name resolution, supplied-value admission, simulation or model admission. | Inspection (TC-1121) |

## Dependencies

- [FR-262](FR-262-evaluate-values-and-calls-at-any-depth.md): the interface,
  fuel and depth behaviour the implementation meets, and AC-1 to AC-8.
- [FR-401](FR-401-host-the-evaluation-interface-in-the-quire-eval-leaf.md):
  the leaf, its dependency set, its closure contract and the reference
  adapter.
- [FR-400](FR-400-separate-checked-terms-from-the-checker.md): the checked
  terms in `quire-checked` and the refusal-record function.
- [FR-096](FR-096-stage-limits-refusal-records-and-readers-carry-a-locus.md):
  the refusal records compared.
- FR-089 (population bindings), FR-106 (witnessed range) and FR-364
  (relationship extents): the environment reads.
- [ADR-011](../decisions/ADR-011-stage-dag-and-dependency-architecture.md)
  §6.1 layer E, FB-14 and §7.3 X-16.
- [AD-001](../assurance/AD-001-shared-evaluation-core-interface.md) slices
  S4 (IR-588) and S6 (IR-590). FR-402 (S7) compares the two implementations
  while both exist.
