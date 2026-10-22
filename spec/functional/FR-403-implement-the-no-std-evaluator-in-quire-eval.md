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
  - target: ix://agent-ix/quire-spec-language/FR-089
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-106
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-364
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-277
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-402
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/AD-001
    type: traces_to
---
# FR-403: Implement the no_std evaluator in quire-eval over checked terms and an environment contract

## Description

[FR-262](FR-262-evaluate-values-and-calls-at-any-depth.md) requires an
implementation of the shared evaluation interface that runs with `core` and
`alloc` only. [FR-401](FR-401-host-the-evaluation-interface-in-the-quire-eval-leaf.md)
places that interface in the leaf `quire-eval` (ADR-011 layer E), which is
`#![no_std]`, has no `std` feature and names no QSL item, and leaves to this
requirement which checked terms the implementation reads.
[FR-400](FR-400-separate-checked-terms-from-the-checker.md) puts those
checked terms in `quire-checked` (layer C).

The crate `quire-eval` SHALL hold the `no_std` evaluator. It evaluates the
`Value` family's checked inputs, a callable application or a checked
expression, over C's checked terms. It admits call arguments itself, reads
the object environment only through an environment contract that
`quire-eval` defines, and is driven by the FR-262 scheduler that
`quire-eval` already holds. `qsl-eval` consumes it. Until FR-405's switch,
`qsl-eval` keeps the std reference evaluator and the std reference adapter;
it keeps the clause and model engines and an implementation of the environment contract over
`qsl-semantics`' `ObjectEnvironment`, and stays a `std` crate. The two
implementations agree on every result, refusal, location, loss and charge,
which [FR-402](FR-402-differentially-test-the-no-std-evaluator-against-the-std-reference.md)'s
differential suite checks while both exist.

This requirement also states which `qsl-semantics` code the evaluator needs
(slice S4, IR-588) and where each part goes. Code and vocabulary that need
only the meter and K, SV and C values move into `quire-eval` or SV, so each
has one definition that both evaluators use. The model reads over an
admitted environment stay in `qsl-semantics` and reach the leaf only through
the environment contract.

## Inputs

- A checked closure: a `quire_checked::CheckedGraph`, whose scope view holds
  each imported library's checked graph keyed by its `PackageId` (FR-400).
- A checked input: a callable `NodeKey` with completed arguments, or a
  `quire_checked::CheckedExpression` with completed parameter values.
- An environment: a value of a type that implements the environment
  contract.
- One `quire_exact::Meter`, one `quire_exact::Cancel`, and optionally a
  supplied-value run (`SuppliedRun`) carrying FR-277's budgets.
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
   never builds one (ADR-011 FB-14). A collection whose iteration order
   reaches an evaluation, a refusal, a loss list, a charge or a reported
   read SHALL be ordered (`BTreeMap`, `BTreeSet`, or a `Vec` in a stated
   order).
3. The evaluator SHALL be one implementation of FR-401's closure contract:
   the callable identity, the admitted signature, the checked input and the
   interpreted transitions of FR-401 items 1 to 4 are answered by
   `quire-eval` itself over C's checked terms, so a registry is created
   directly from a checked closure with no QSL adapter in between.

### Scope: the `Value` family

The `no_std` evaluator SHALL evaluate exactly FR-262's checked inputs: an
application of an admitted callable and a checked expression with its
parameters. It has no entry that evaluates a state clause, a clause set, an
operation frame, a separating `forall` witness (ADR-031), a model type's
invariants by runtime value path (FR-366), a temporal, protocol or hyper
clause, or a simulation step. Until FR-405's switch, those entries stay in
`qsl-eval` and run on the std reference evaluator; the switch moves their
evaluation onto a `quire-eval` clause entry
([FR-405](FR-405-retire-the-std-reference-evaluator-for-quire-eval.md)
"Clause evaluation"), and this section then scopes the `Value`-family entry.

The checker's body entry (`check_expression`) and a function body admit
every `NodeKind` except five, which only other checker passes build:

- `NodeKind::Pre`, built only for `pre(...)` in an operation's
  postcondition (`check_postcondition_expression`); every other pass
  refuses `pre(...)` as `forbidden-pre-read`;
- `NodeKind::NamedPopulation`, built only when state-clause or
  temporal-clause typing names a population (FR-391);
- `NodeKind::PopulationObjects` (FR-139) and `NodeKind::History` (FR-138),
  built only in a refinement pass;
- `NodeKind::Dispatch` (FR-151 `receiver.member(args)`), refused outside
  the invariant, precondition and postcondition passes. It reaches an
  evaluator through `check_clause_expression` or a dispatch candidate's
  precondition.

The `no_std` evaluator's scope is defined by node kind. It SHALL evaluate
`Dispatch` and every node kind outside the five, and fault on the other
four (`Pre`, `NamedPopulation`, `PopulationObjects`, `History`). A `CheckedExpression` does not record which checker
entry built it, so an expression from `check_postcondition_expression` that
contains no `pre(...)` evaluates normally. Begin over a checked expression
that contains one of the four node kinds SHALL return `CallFailure::Fault`
with an `InternalFault` whose stage is `quire-eval` and whose invariant is
`outside-value-family`, with no charge and no body invocation. A function
body cannot contain them, so no callable application reaches that fault.
The reference evaluates the four kinds, so FR-402 and this requirement
compare the two evaluators only on inputs without them.

A protocol reference (`protocol <name>`, `NodeKind::ProtocolReference`,
FR-218) can occur in any body, including a function's. Both evaluators
SHALL refuse it when evaluation reaches it, with
`FamilyResult::Refused(ProtocolReferenceUnsupported)`: catalog code
`unknown_required_feature`/`unsupported-feature`, field `construct` =
`protocol`, located at the reference (FR-308).

The separation and stop-report vocabulary that clause evaluation builds
stays where it is: `qsl_foundation::witness`'s `InvocationBinding`,
`ObservationIdentity`, `RuntimeValuePath`, `SeparationStep`,
`ValuePathStep` and `ValuePathSubject`, and `model::observation`'s
`DocumentRef`. The `no_std` evaluator builds no trail, provenance, stop
report or runtime value path, so it names none of them.

### The checked closure and checked input it reads

1. A registry SHALL be created from exactly one checked closure, a
   `CheckedGraph`, which it holds for its lifetime (FR-401 item 5).
2. The admitted callables of that closure are the functions of the graph
   and of each imported graph its scope view holds that are callable by
   name. Each is identified by its `CallableFunction` `NodeKey`, with the
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

The checked input SHALL reach `quire-eval` in process: the consumer holds
the checked graph and passes it to the registry. In QSL it is the admitted
`CheckedPackage`'s checked graph. RT and CG take `quire-eval` as an
in-process crate dependency and need no wire form of a checked graph; CG's
only wire form for evaluation is the `quire-eval-corpus` case (FR-402).
This requirement therefore specifies the in-process handoff only. A
backend never builds a checked term: FR-400 and ADR-011 FB-14 forbid a
backend from enabling `quire-checked`'s `construct` feature.

### What the evaluator serves for RT and CG

The `Value`-family scope is what RT's ported function application covered:
checked function application over `Value`-family expression bodies,
argument planning before any charge (RT's `plan_call` and
`plan_evaluation`), resumable bodies whose calls run on explicit call
frames (RT's `Frame` and `Body`, FR-262's session, body contract and child
request), and the `work_units` charge sequence with its ordered admitted
charges. RT and CG evaluate no state clause, operation frame or other
clause, so they use only the `Value`-family entry; until FR-405's switch no
clause entry is part of `quire-eval`.

### Argument admission

`quire-eval` SHALL admit a callable's completed arguments, and a checked
expression's completed parameter values, before the application's first
charge, with no meter in scope. Both evaluators run this one admission: the
reference evaluator calls it in place of its own. For the declared
parameter `ValueType`s of the callee, in order, it decides:

1. **Arity.** A count that differs from the declared count refuses
   `InputRefusal::Arity` with both counts.
2. **Population parameters.** A `Population` argument whose `PopulationId`
   the environment does not resolve, or resolves to a binding whose
   declared maximum differs from the parameter's (absence included),
   refuses `InputRefusal::WrongValueKind` naming the parameter (FR-089-AC-5).
3. **Membership.** Every other argument is admitted against its parameter's
   `ValueType` in the type environment of the checked graph's scope view, by
   the supplied-value run's admission phase (`SuppliedRun::admit`, FR-277).
   That phase charges the run's admission budget, not the meter.
4. **References.** Unless the environment marks its references unresolved,
   every object reference inside an argument that the environment's object
   closure does not contain refuses `InputRefusal::DanglingReference`
   naming the parameter.
5. **Cancellation.** The run polls its `Cancel` for each value it visits.

A failure is a pre-charge Begin failure: `CallFailure::Input`,
`CallFailure::Cancelled`, `CallFailure::Supplied`,
`CallFailure::UnsupportedHelper` or `CallFailure::Fault`, from the run's
`SuppliedFailure` conversion (FR-401), with no charge and no body
invocation. A Begin with no supplied-value run uses a run of its own at
`SuppliedLimits::default()` with a fresh `Cancel` that nobody holds, as the
reference's `call` and `evaluate` do. Argument admission therefore never
observes the session's `Cancel`. The session's `Cancel` is polled from the
application's first charge, as FR-262 states.

The supplied-value run (`SuppliedRun`, `SuppliedLimits`, and the private
event and storage helpers FR-401 places with them) SHALL move from
`qsl-semantics`' `value::supplied` to SV's `supplied_stop` module, beside
the stop records it produces. It names only SV, K and the catalog
vocabulary FR-401 moves to SV. Its test-only reservation permit is behind
SV's `test-support` feature, which only a `[dev-dependencies]` entry
enables. No `qsl_semantics::` path to it remains. The conversion phase that
turns supplied JSON values into kernel values (FR-098) stays in QSL and
charges the same run before admission.

### The environment contract

`quire-eval` SHALL define an environment contract: the reads a
`Value`-family evaluation and its argument admission make of the objects and
populations they run over. Its answers are C, SV and K values or boxed
`CatalogCoded` / `UndefinedCoded` causes (SV); it names no QSL type. The
reads are the nine the std reference evaluator makes of `ObjectEnvironment`
for `Value`-family inputs:

1. the SV `ObjectClosure`;
2. whether the environment's references are unresolved;
3. whether a field read of an object is witnessed out of its declared range
   (FR-106 check 6.5);
4. the FR-364 relationship extents: whether a relationship's extent is
   complete, its instances, and the manifest a `reaches` query resolves
   through;
5. the size of a population that an authored population bound names
   (FR-380), read on the navigation path;
6. for a `PopulationId`, whether a binding is recorded and its declared
   maximum;
7. an `allInstances` query over a recorded `PopulationId`, answered under
   the session meter, with the members in ascending reference-key order;
8. a `lookup` query over a recorded `PopulationId`, answered under the
   session meter;
9. a report of each field read and population-membership read, with the
    object, owning effective type and attribute name, or the population's
    declaration key and the one object looked up.

Items 7 and 8 return a value, a stop, an internal fault, an absent key or an
uncovered subtype set, or a refusal as a boxed `CatalogCoded` cause. The
contract SHALL NOT expose a way to change the environment. Population and
model admission (`admit_binding`, `admit_invocation`, the observation
admission and model normalization) happen before Begin, outside the
contract.

`quire-eval` SHALL provide one implementation of the contract over an SV
`ObjectClosure` alone, with no population recorded, no relationship
extents, no witnessed range, no population size and no read report. For every read it SHALL give the answer `ObjectEnvironment::new`
over the same closure gives. A `Population` argument therefore refuses at
argument admission as `CallFailure::Input(InputRefusal::WrongValueKind)`
naming the parameter, as in the reference.

`qsl-eval` SHALL implement the contract over `qsl-semantics`'
`ObjectEnvironment`. Items 7 and 8 SHALL call `qsl-semantics`'
`value::model_query` (`evaluate_lookup`, `evaluate_all_instances`) with the
session meter, so their charges and their order are the reference's. A
`ModelRefusal` they return reaches the leaf boxed as the `StateModel`
model-query refusal `ModelQueryRefusal`, which stays in `qsl-eval`. Item 9 records an `ObservedRead`
in the environment's `ReadLog` when one is attached, under the observation
the reference records for a `Value`-family evaluation:
`Observation::Current` for a field read, and `Observation::Post` for a
population-membership read. The evaluation's anchor starts at `Post`, and
only a `pre(...)` node switches it.

### Which `qsl-semantics` code the evaluator needs (S4)

The evaluator-needed code is the set of shipped items outside
`quire-checked`, SV and K that the reference evaluator's `Value`-family
path names: in `qsl-semantics`, and in `qsl-eval`'s own causes once
`quire-eval` raises them too. Each part SHALL have exactly one of these
dispositions:

| Code | Disposition |
| --- | --- |
| The checked terms, `DeclarationKey`, `Observation` and the other values FR-400 lists | In C (FR-400); no S4 change |
| `value::definition::divide` and `value::definition::modulo`, which need only an `AdmittedIntegerDivision` (C), K's integers and the meter | Move to `quire-eval`. Both evaluators call them there |
| `model::refusal::render_identity`, the rendering of member identity bytes | Moves to SV's `catalog` module. Its `qsl-semantics` callers (population lookup) and both evaluators call it there |
| The `StateModel` catalog prefix (`"state-model"`, which `FamilyKind::StateModel.catalog_code_prefix()` returns) | The string moves to SV's `catalog` module as a constant. `FamilyKind`, which stays in `qsl-semantics` (FR-401), returns that constant for `StateModel`, and the causes in `quire-eval` prefix with it |
| `qsl-eval`'s `Value`-family runtime causes: `PreconditionFailure`, `StateModelUndefined` (`PreconditionFalse`, `AbsentKey`, `IncompleteClosure`, `OnlyNotOne`), `NavigationRefusal`, `ReferenceUnresolved` and `ProtocolReferenceUnsupported` | Move to `quire-eval`. Both evaluators raise them there, so each catalog code, prefix, field set and rendering has one definition |
| `qsl-eval`'s clause and refinement causes: `HistoryUnresolved`, `NamedPopulationIncomplete`, `ProtocolClauseSnapshot`, `ProtocolClauseUnsupported`, `ProtocolClauseFrameRefusal` (which carries `model::observation::AdmissionRecord`) and `ModelQueryRefusal` | Stay in `qsl-eval`, raised by clause evaluation or built by `qsl-eval`'s environment implementation |
| `ModelRefusal` (`model::normalize`) and `ModelRefusalCause` (`model::refusal`), including `ModelRefusal::type_mismatch` and `absent_key`, which the reference raises for FR-391 named-population lookups | Stay in `qsl-semantics`. Named populations are clause-only, and an item-7 or item-8 refusal reaches the leaf boxed |
| `value::supplied`'s `SuppliedRun`, `SuppliedLimits` and their helpers | Move to SV's `supplied_stop` module (Argument admission) |
| `model::object_environment` (`ObjectEnvironment`, `ObservedRead`, `ReadLog`), `model::population` (`PopulationBinding` and its queries), `value::model_query` and `model::observation`'s admitted-observation types | Stay in `qsl-semantics` with `std`. The leaf reaches them only through the environment contract that `qsl-eval` implements. `PopulationBinding` holds a domain package and a `ModelIndex`, which are not leaf vocabulary (FR-401) |

No `qsl_semantics::` or `qsl_eval::` path to a moved item remains, and no
item is re-exported (ADR-011 §7.2). `qsl-semantics` gains no `std` feature
from this slice.

The `HashMap` in `model::population`'s subsetting check (`member_by_object`,
built during `admit_binding`) is read by key only. The admission walks
`admitted` in ascending reference-key order, so the map's order reaches no
refusal, and admission refuses the same member for the same document on
every run. It is admission code, which stays `std`.

### Results, refusals and accounting

For every `Value`-family checked input (an admitted callable's application,
or a checked expression from the body entry), the `no_std` evaluator SHALL
return the reference's results:

1. **Evaluation.** The same `FamilyOutcome<Value>` arm and value, the same
   location and the same loss list in the same order. A family refusal or
   undefined result has the same catalog code, cause tag, catalog fields
   and category.
2. **Call failure.** The same `CallFailure` arm: `Input` with an equal
   `InputRefusal`, `Cancelled` with an equal `CancelCause`, `Storage` with
   the same parameter and failure, `Supplied` and `UnsupportedHelper` with
   equal stops. `Fault` is compared by arm and category.
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
NFR-015 row. Until FR-405's switch, it keeps the items below; FR-405
"The state after the switch" states what it keeps after it:

- the std reference evaluator (`value::expression`'s `Machine`), which
  `CheckedPackageEvaluation`'s `call` and `evaluate` use by default while
  it remains the reference;
- the clause, clause-set, frame, separation and value-invariant entries,
  the S6a family seam over the `qsl-semantics` family marker types, and
  the simulation, automaton, hyper, temporal, fairness and model-subject
  engines, all on the reference evaluator until FR-405's switch;
- until FR-405's switch, the std reference adapter of FR-401; and the environment contract
  implementation over `ObjectEnvironment`;
- name resolution (`QualifiedName`), the FR-098 conversion phase before
  `call_supplied`'s admission, and the refusal-record function.

Until FR-405's switch, `qsl-eval` SHALL expose an entry that runs a checked package's call or
expression through the `quire-eval` evaluator, with the package's checked
graph as the closure and its `ObjectEnvironment` as the environment. The
FR-402 differential runs and this requirement's FR-262 runs use that entry.
`qsl-eval` re-exports no `quire-eval` item.

The host-body registry, its admission result, the session, its step, the
body contract and the child request stay in `quire-eval` (FR-401), so
FR-262-AC-3, AC-4 and AC-6 to AC-8 exercise the same scheduler under both
implementations.

### FR-262 with `std` off

In QSL, FR-262-AC-1 to FR-262-AC-8 SHALL each also run with the `no_std`
evaluator as the implementation. QSL's checker admits the source and
`qsl-package` links the checked package with `std`, then the `quire-eval`
evaluator evaluates its checked graph. Each run SHALL meet that criterion's
expectations with the reference's values, and FR-262-AC-1 and AC-2 run on a
thread with a 512 KiB stack.

These runs, and FR-402's, are host test builds of QSL. Cargo unifies
features across that build, so `quire-eval`'s dependencies are built with
features QSL enables (`quire-checked`'s `std` and `construct`, and
`quire-exact`'s `test-support`). `quire-eval`'s own source is the same in
every build, because it has no `std` feature. The evidence that it builds
without `std` is FR-401-AC-1's `thumbv7em-none-eabi` gate in its own
repository, and nothing else.

## Acceptance Criteria

Each criterion below that compares the `no_std` evaluator with the
reference (AC-2 to AC-7 and AC-10 to AC-14) holds as written until FR-405's
switch. After the switch it holds in this form: the test runs once, on
`quire-eval`, and the reference's results are its recorded expected
values. AC-3's and AC-6's reference-side registration and call, and AC-7's
second pass, become that single run. AC-9's reference clause is bounded by
the switch, and FR-405-AC-4 states what holds after it.

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-403-AC-1 | `quire-eval`'s manifest names only `quire-checked`, `quire-semantic-value`, `quire-exact`, `quire-walk` and `thiserror` as normal dependencies, with default features off, and `cargo tree -p quire-eval -e features -i quire-checked` in `agent-ix/quire-eval`'s gate shows no `construct` feature. It declares no feature that adds or removes the evaluator. | Test (TC-1121) |
| FR-403-AC-2 | `qsl_semantics::value::divide`, `qsl_semantics::value::modulo`, `qsl_semantics::model::refusal::render_identity` and `qsl_semantics::value::supplied::SuppliedRun` fail to compile from another crate, and each resolves at its new path. With a package's admitted integer division law, `-7 div 2` and `-7 mod 2` evaluated through each evaluator return the value and admitted charges that `quire_exact::divide` and `quire_exact::modulo`, called directly with that law's profile and a fresh meter, return. | Test (TC-1121) |
| FR-403-AC-3 | Over FR-401-AC-5's fixture (a package with `f(x: Int[0, 9]): Boolean`, an FR-151 dispatched member call, and a dependency with `g(): Integer`), a registry the `no_std` evaluator creates from the package's checked graph returns `Accepted`, `SignatureMismatch`, `Accepted`, `CallableNotAdmitted` and `CallableNotAdmitted` for the five registrations FR-401-AC-5 lists, equal to the reference adapter's results. | Test (TC-1121) |
| FR-403-AC-4 | For each vector, the `no_std` evaluator and the reference return equal results, equal FR-096 refusal records, equal consumed counters and equal ordered admitted charges. `count(3)` completes with `3`, with four `function.call` charges among its admitted charges. `function d(x: Int[0, 9]): Integer pure { 10 div x }` called with `0` returns the reference's non-completed outcome, located at the `div` node. `f` called with `10` returns `CallFailure::Input` from `quire-eval`'s argument admission, with no charge. `f` called with two arguments returns `CallFailure::Input(InputRefusal::Arity)`, with no charge. A call begun with an already-cancelled `Cancel` returns `CallFailure::Cancelled`, with no charge. `count(3)` with `work_units` one below its measured spend returns `Incomplete` naming `work_units` at the reference's denied charge point, with that charge unrecorded and the earlier spend kept. | Test (TC-1121) |
| FR-403-AC-5 | Over an `ObjectEnvironment` with one population of three recorded members and an attached `ReadLog`, a function whose body is an `allInstances` over its `Population` parameter, evaluated through the `no_std` evaluator with `qsl-eval`'s environment implementation, returns the members in the reference's order, with the reference's charges, and leaves the same `ObservedRead` set in the `ReadLog`. | Test (TC-1121) |
| FR-403-AC-6 | Over `quire-eval`'s closure-only environment, a call of a function with a `Population` parameter, given a `Population` value whose `PopulationId` is not recorded, returns `CallFailure::Input(InputRefusal::WrongValueKind)` naming parameter `0`, with no charge. The reference over `ObjectEnvironment::new` of the same closure returns the same result. | Test (TC-1121) |
| FR-403-AC-7 | FR-262-AC-1 to FR-262-AC-8 each pass a second time with the `no_std` evaluator as the implementation, through `qsl-eval`'s `quire-eval` entry, with the reference's expected values. The AC-1 and AC-2 runs use a thread with a 512 KiB stack and observe no depth refusal and no stack overflow. | Test (TC-1121) |
| FR-403-AC-8 | Admitting a population document whose three members are listed in two different orders, with a subsetting violation on two of them, refuses both times naming the violating member with the lowest reference key, on each of ten runs. | Test (TC-1121) |
| FR-403-AC-9 | `qsl-eval`'s manifest declares no `std` feature. Until FR-405's switch, `CheckedPackageEvaluation::call` and `evaluate` run the reference evaluator. `qsl-eval` re-exports no `quire-eval` item and defines no argument admission of its own. `quire-eval`'s source contains no name resolution, FR-098 conversion phase, model admission or simulation, and, until FR-405's switch, no clause, frame, separation or value-invariant entry. | Inspection (TC-1121) |
| FR-403-AC-10 | On each of ten runs, the `no_std` evaluator's results equal the reference's in order: the members of AC-5's `allInstances` over a population recorded with its members in descending reference-key order, which both return ascending; and the loss list of FR-262-AC-8's kernel-loss case. | Test (TC-1121) |
| FR-403-AC-11 | The `StateModel` undefined result of a `lookup<T>(p, r) absent undefined` with an absent key, raised by each evaluator, has the catalog code prefix `state-model` and the `binding` and `key` fields `render_identity` gives. `FamilyKind::StateModel.catalog_code_prefix()` returns SV's constant, and `qsl-eval`'s source defines no `StateModelUndefined`, `PreconditionFailure`, `NavigationRefusal`, `ReferenceUnresolved` or `ProtocolReferenceUnsupported`. | Test (TC-1121) |
| FR-403-AC-12 | Through each evaluator, a checked expression `x + 1` with one parameter `x: Int[0, 9]`, given `10`, returns `CallFailure::Input` from argument admission with no charge; given `3`, it completes with `4` and no `function.call` among its admitted charges. A chain of three functions with no parameters, whose bodies call the next or, for the last, return a literal, admits exactly three `function.call` charges with `work_units` spend `3`; with `work_units` `2` it returns `Incomplete` naming `work_units` at the third `function.call`, with two calls in the counter and the log. | Test (TC-1121) |
| FR-403-AC-13 | A checked expression built by `check_postcondition_expression` with a parameter `p: Population<T>[3]` and body `pre(allInstances<T>(p))`, begun on the `no_std` evaluator, returns `CallFailure::Fault` with stage `quire-eval` and invariant `outside-value-family`, with no charge and no body invocation. Over an `ObjectEnvironment` that records `p`'s population with three members, the same expression without `pre(...)` gives equal results from both evaluators: once built by `check_expression`, and once by `check_postcondition_expression`. | Test (TC-1121) |
| FR-403-AC-14 | `function refers(): Boolean pure { protocol Fill }`, in a unit that declares the protocol `Fill`, called through each evaluator, returns `FamilyResult::Refused` with catalog code `unknown_required_feature`/`unsupported-feature`, field `construct` = `protocol`, and the protocol reference's location, with equal FR-096 records and equal admitted charges. | Test (TC-1121) |

## Dependencies

- [FR-262](FR-262-evaluate-values-and-calls-at-any-depth.md): the interface,
  fuel and depth behaviour the implementation meets, and AC-1 to AC-8.
- [FR-401](FR-401-host-the-evaluation-interface-in-the-quire-eval-leaf.md):
  the leaf, its dependency set, its closure contract and the reference
  adapter.
- [FR-400](FR-400-separate-checked-terms-from-the-checker.md): the checked
  terms in `quire-checked` and the refusal-record function.
- [FR-402](FR-402-differentially-test-the-no-std-evaluator-against-the-std-reference.md):
  the differential suite that compares the two implementations.
- [FR-096](FR-096-stage-limits-refusal-records-and-readers-carry-a-locus.md):
  the refusal records compared.
- [FR-277](FR-277-bound-every-lifecycle-operation-by-caller-limits.md):
  the supplied-value run and its admission budget.
- [FR-089](FR-089-carry-population-identity-across-the-kernel-boundary.md),
  [FR-106](FR-106-admit-snapshots-and-invocations.md),
  [FR-364](FR-364-carry-relationship-instances-in-an-observation-and-read-them-for-navigation.md)
  and [FR-380](FR-380-bound-a-domain-package-population-and-link-a-reachability-to-its-node.md):
  the environment reads.
- [ADR-011](../decisions/ADR-011-stage-dag-and-dependency-architecture.md)
  §6.1 layer E, FB-14 and §7.3 X-16.
- [AD-001](../assurance/AD-001-shared-evaluation-core-interface.md) slices
  S4 (IR-588) and S6 (IR-590).
