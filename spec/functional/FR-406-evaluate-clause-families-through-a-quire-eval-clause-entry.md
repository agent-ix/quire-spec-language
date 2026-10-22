---
id: FR-406
title: "Evaluate the clause families through a quire-eval clause entry and a clause environment contract"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-262
    type: implements
  - target: ix://agent-ix/quire-spec-language/FR-403
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-401
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-400
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-107
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-265
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-268
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-031
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-138
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-139
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-366
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-391
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-106
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-402
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-405
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/NFR-015
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/AD-001
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: traces_to
---
# FR-406: Evaluate the clause families through a quire-eval clause entry and a clause environment contract

## Description

[FR-403](FR-403-implement-the-no-std-evaluator-in-quire-eval.md) gives
`quire-eval` one entry, the `Value`-family Begin over a callable application
or a checked expression. That Begin returns the `outside-value-family` fault
for `NodeKind::Pre`, `NamedPopulation`, `PopulationObjects` and `History`.
Every other evaluation `qsl-eval` performs runs a checked body over admitted
observations: an FR-107 state clause with its FR-265 trail, the ADR-031
separation check (FR-268), an FR-366 value invariant, an FR-138 history
update, an FR-139 refinement row, an FR-124 terminal predicate, an FR-167
state predicate, an ADR-027 protocol slot, an FR-187 reward and an FR-358
closed subexpression. [FR-405](FR-405-retire-the-std-reference-evaluator-for-quire-eval.md)
retires the std reference evaluator only when all of these run on
`quire-eval`.

This requirement defines the second entry of the one `quire-eval`
evaluator, the clause entry, and the clause environment contract it reads.
The clause entry evaluates one checked body over a frame of slots. It
evaluates every node kind, the four above included. It reads two
observations, the body's own and an optional pre observation, and reports
the FR-265 trail as events to a recorder the caller supplies. `quire-eval`
names no QSL type. The trail, the witness record and the separation check's
steps stay in `qsl-eval`, which assembles them from the events and the
results. The `Value`-family entry, its environment contract and its fault
are unchanged, so RT and CG build and call `quire-eval` as before.

## Inputs

- A registry created from one checked closure (FR-403), and one
  `quire_exact::Meter` with its optional `Cancel` handle (FR-262).
- A clause input, in one of two forms:
  - **Body form.** A checked node of the registry's closure graph, the
    body's slot count, and a frame: one entry per slot, a value or unbound.
  - **Expression form.** A `quire_checked::CheckedExpression` with
    completed parameter values.
- A regime, `Value` or `Clause`.
- The read map: the checked term's per-read observation, a map from a
  node's `Location` to C's `Observation` (`Current`, `Pre` or `Post`).
  It is empty for a body whose reads all observe the body's own
  observation.
- The receiver correspondence, optional: the reference `self` has in the
  body's own observation and, optionally, its reference in the pre
  observation.
- An environment: a value of a type that implements the clause environment
  contract.
- A trail recorder, optional: a value of a type that implements the trail
  event contract.

## Outputs

- A terminal `quire_eval::Evaluation` or `quire_eval::CallFailure` from
  an FR-262 session, with the meter's spend.
- The trail events, reported to the recorder in evaluation order.
- The reads reported to the environment through the contract.

## Behavior

### One evaluator, two entries

1. The clause entry SHALL be part of the crate `quire-eval`. It SHALL share
   the `Value`-family entry's node dispatch, FR-262 scheduler, task stack,
   meter and charge points. Begin over a clause input SHALL return an
   FR-262 session that steps like a `Value`-family session, with the same
   child requests, cancellation and resumption.
2. `quire-eval` SHALL have no feature that adds or removes the clause
   entry. Its `[dependencies]` SHALL stay FR-403's set, and it SHALL build
   for `thumbv7em-none-eabi` with the clause entry in place (FR-401-AC-1).
3. `quire-eval` SHALL name no QSL item. The clause input, the read map, the
   receiver correspondence and every event payload are C, SV and K values.
   The clause environment contract and the trail event contract are traits
   `quire-eval` defines. Their implementations over QSL types are in
   `qsl-eval`.
4. The `Value`-family Begin SHALL keep FR-403's inputs, argument admission,
   environment contract (items 1 to 9) and results. Over a checked
   expression that contains `Pre`, `NamedPopulation`, `PopulationObjects` or
   `History`, it SHALL return the `outside-value-family` fault with no charge
   and no body invocation. The clause environment contract extends FR-403's
   contract: a type that implements only FR-403's contract, such as
   `quire-eval`'s closure-only environment or RT's, needs no new item and
   compiles unchanged.

### The clause input

1. **Body form.** The node SHALL be a node of the registry's closure graph:
   a clause, row, predicate, invariant, reward or protocol-slot body, or a
   subnode of one. A node of another graph SHALL make Begin return
   `CallFailure::Fault` with stage `quire-eval` and invariant
   `clause-root-outside-closure`, with no charge. The frame is extended
   with unbound slots to the slot count. The clause entry does not admit
   frame values: they come from admitted observations (FR-106) or from an
   earlier session's result.
2. **Expression form.** Begin SHALL admit the parameter values by FR-403's
   argument admission, with the same pre-charge failures, and then
   evaluate the expression's root with the admitted values in its first
   slots.
3. **Regime.** The regime selects the read and charge rules below. `Clause`
   is the regime of FR-107's `ProtocolClause` family: the state-clause,
   terminal-predicate, state-predicate, refinement-row, history-update,
   protocol-slot and value-invariant bodies, and the separation check's
   evaluations. `Value` is the regime of a body that reads no observation:
   a reward, a closed subexpression, and a checked expression that
   `CheckedPackageEvaluation::evaluate` routes here.

### Reads, charges and results per node kind

The clause entry SHALL give each node kind the reference's result, charges
and reported reads. For the node kinds below this section states them. A
node kind not listed evaluates as on the `Value`-family entry.

| Node | Environment read | Charges | Result and stops |
| --- | --- | --- | --- |
| `Pre(e)` | none | none | Evaluates `e` with the population anchor set to `pre`, restores the anchor afterwards, and returns `e`'s value. A callee's body starts with the anchor at `post`. |
| A field read `e.f` of an object, `Clause` regime | contract item C2 on the side the read map gives the node: `pre` when it maps the node to `Pre`, otherwise `own` | `model.deref` for an explicit `deref(...)`, then `model.navigate`, before the read | When the node reads `pre` and the read's reference is the receiver correspondence's own-side reference, the pre-side reference is read. A `pre` read with no pre side returns `FamilyResult::Refused(ProtocolClauseSnapshot::WrongAnchor)`, code `wrong_snapshot`/`wrong-anchor`, located at the read. |
| A field read, `Value` regime | FR-403 items 1 to 3 and 9 on the `own` side | as on the `Value`-family entry | as on the `Value`-family entry |
| `allInstances` or `lookup` over a `Population`-typed operand, `Value` regime | FR-403 items 6 to 8 at the population anchor: at `post`, as FR-403 states; at `pre`, contract item C5 | as FR-403 items 7 and 8 | At `pre`, a binding with no pre anchor returns `FamilyResult::Refused(ProtocolClauseSnapshot::WrongAnchor)`. |
| `allInstances` or `lookup` over a `Population`-typed operand, `Clause` regime | none | none | `FamilyResult::Refused(ProtocolClauseUnsupported)` (`AllInstances` or `Lookup`), code `unknown_required_feature`/`unsupported-feature`, located at the node. |
| `allInstances<T>(P)`, `lookup<T>(P, r)` or a domain over a `NamedPopulation` `P` (FR-391) | C3 on the node's read-map side | `allInstances`: one `population.visit` per listed member, then `collection.bound` sized by the selected count. `lookup`: `lookup.key`, then `lookup.result-retain` sized 2 for a present `empty`-mode result and 1 otherwise | No declared member type covering `T` stops with FR-084's incomplete-closure result. A population not listed `complete` returns `FamilyResult::Refused(NamedPopulationIncomplete)`, code `incomplete_population`/`incomplete-scope`, field `population`. A reference not conforming to `T`, and an absent key under `refused`, return the boxed refusal contract item C6 builds. An absent key under `undefined` returns the `StateModel` undefined result `AbsentKey` (FR-403). `allInstances` returns the conforming members in C3's order. |
| `PopulationObjects` (FR-139) | C3 on the node's read-map side | the charges of forming the result collection | The population's members, in C3's order. An unlisted population is an internal fault, `population-members-unobserved`. |
| `History` (FR-138) | C4 on the node's read-map side | none | The history value. A field with no value returns `FamilyResult::Refused(HistoryUnresolved)`, code `missing_declaration`/`missing-name`, field `history`. |

After every population or membership read above, the clause entry SHALL
report one membership read through contract item C7, under the observation
the population anchor gives (`Pre` or `Post`). After every field read, it
SHALL report one field read through C7, under the observation the read map
gives the node (`Current` when the map has no entry).

Every other rule is the `Value`-family entry's: short-circuit of `and`,
`or` and `implies`, the FR-364 navigation reads, the FR-380 population
bound, the FR-106 check 6.5 witnessed reads, the `reaches` charges of
FR-107, the meter's denial as a located `Incomplete`, and evaluation with
no depth setting and no native recursion that grows with the input
(FR-262, ADR-030).

### The clause environment contract

`quire-eval` SHALL define the clause environment contract as an extension
of FR-403's environment contract. FR-403's items 1 to 9 are answered for the
`own` side. The extension adds these reads:

1. **C1, sides.** Whether a pre side exists.
2. **C2, side reads.** FR-403's items 1 to 5 for a given side, `own` or
   `pre`.
3. **C3, named and concrete populations.** For a side and a population's
   `DeclarationKey`: whether the observation lists the population
   `complete`, and its members in ascending reference-key order, or that
   the population is not listed.
4. **C4, history.** For a side, an object reference and a history field's
   name: the field's value, or that it has none.
5. **C5, pre-anchored population reads.** For a `PopulationId`: whether the
   binding has a pre anchor, and FR-403's items 7 and 8 answered over that
   anchor under the session meter, with FR-403's answer forms.
6. **C6, model-query refusals.** For a type mismatch (the reference's
   object type and the queried type, each as its `DeclarationKey` node) or
   an absent key (the population's `DeclarationKey` node and the key's
   identity bytes): a boxed `CatalogCoded` cause.
7. **C7, read report.** FR-403's item 9, carrying the side and the
   observation stated above.

The contract SHALL NOT expose a way to change the environment.
Observation admission (FR-106), frame check 11 (FR-115), model
normalization and population admission happen before Begin, outside the
contract.

`qsl-eval` SHALL implement the clause environment contract over two
`ObjectEnvironment`s, the body's own and the optional pre. C3 and C4 answer
from `population_complete`, `population_members` and `history`. C5 calls
`value::model_query` over the binding's pre anchor with the session meter.
C6 builds `ModelQueryRefusal` from `ModelRefusal::type_mismatch` and
`ModelRefusal::absent_key`. C7 records an `ObservedRead` in the side's
`ReadLog` for a field read, and in the own side's `ReadLog` for a
membership read, when one is attached.

### The trail event contract

`quire-eval` SHALL define the trail event contract. When Begin is given a
recorder, the clause entry SHALL report each event below to it at the point
of evaluation stated, in evaluation order. Each event returns either
success or an `InternalFault`. A fault SHALL end the session with
`CallFailure::Fault` carrying that fault, with no later charge. Without a
recorder, the clause entry reports no event, and its results, charges and
reported reads SHALL be those of the same Begin with a recorder that
accepts every event.

The clause level is the outermost frame while no query or fold is in
progress and no imported library body is running. The clause frame is the
outermost frame while no imported library body is running.

| Event | Reported when | Payload |
| --- | --- | --- |
| `decided` | at the clause level, the left operand of an `and`, `or` or `implies`, or the condition of an `if`, evaluated to a Boolean | the node's location, the Boolean |
| `built` | at the clause level, a collection literal formed a collection | the node's location, the collection |
| `member` | at the clause level, a field read of an object read a collection, record or optional value | the node's location, the object reference, the member name, the side, the value |
| `selection` | at any level, a field or member selection met its charges and read its value | the selection node, its operand node, the step (`field` of a record or `attribute` of an object) with its name, the object an attribute read, and whether the selection is in the clause frame |
| `element` | in the clause frame, a query or fold bound an element of its domain to its binder slot | the domain's source node, the domain collection, the slot, the element's position |
| `bound` | at any level, a `let`, a `case` payload or a fold's accumulator or binder bound a slot | the slot |
| `field` | at the clause level, a projection of field `i` of a record read a collection, record or optional value | the record, `i`, the value |
| `unwrapped` | at the clause level, `value(e)` read a present payload that is a collection, record or optional value | the optional value, the payload |
| `converted` | at the clause level, a collection conversion formed a collection | the source collection, the kept elements in source order, the result |
| `computed` | at the clause level, a `map` or `filter` formed its result | the source collection, the source node, each result's source position, the results, the formed collection |
| `stopped` | at the clause level, a `forall` stopped at an element whose body is `false`, or an `exists` at one whose body is `true`, before its last element | the quantifier node, the domain collection, the element's position, the element |

A value in a payload is the evaluator's own value, so a recorder can tell
two reads of one stored collection apart by identity (`Arc` pointer
equality), as the reference's trail does.

`qsl-eval` SHALL implement the trail event contract as its FR-265 trail
recorder. From the events, the clause's observation identities, the
receiver and the slot bindings' subjects, it builds the `Trail`, and from
it the FR-107 `ClauseEvaluation`: `reads()`, `decision()`, `stop()` with
each `StopReport`'s occurrence key, index and runtime value path, and
`receiver_path()`. The FR-265 derivation (ADR-031 SW-2 to SW-6) reads that
`ClauseEvaluation` and is unchanged. The `StopReport`, `ClauseRead`,
`Provenance`, `RuntimeValuePath`, `ValuePathSubject`, `ValuePathStep`,
`InvocationBinding`, `ObservationIdentity` and `OccurrenceKey` stay in
`qsl-eval` and `qsl-foundation`. A recorder failure is the reference's
fault at the same event: `quantifier-has-an-occurrence` or
`domain-has-an-occurrence` at a `stopped` event.

### The separation check (ADR-031 SW-13)

`qsl-eval` SHALL run FR-268's separation check as clause-entry Begins in
the `Clause` regime over one meter, with the clause's bindings in the
frame's first slots and a recorder:

1. one Begin for each `let` on the decision path, outermost first, whose
   value `qsl-eval` binds into the frame for the next Begin;
2. one Begin over the quantifier's domain, from whose events the recorder
   gives the domain's provenance;
3. no Begin for the element step: `qsl-eval` selects the element whose
   provenance position is the record's `index` and compares its value path
   and value with the record, evaluating nothing;
4. one Begin over the quantifier's body with the binder slot bound to that
   element.

A Begin whose evaluation completes no value ends the check at its step,
with the result FR-268 states for that step. The check's result and admitted
charges SHALL equal the reference's.

### Causes

The clause causes that the node dispatch raises SHALL be defined in
`quire-eval` and raised there by both evaluators, so each catalog code,
field set and rendering has one definition: `HistoryUnresolved`,
`NamedPopulationIncomplete`, `ProtocolClauseSnapshot` (over SV's
`WrongSnapshotCause`) and `ProtocolClauseUnsupported`. No `qsl_eval::` path
to them remains, and none is re-exported (ADR-011 §7.2).

These causes stay in `qsl-eval`:

- `ModelQueryRefusal`, which wraps `qsl-semantics`' `ModelRefusal` and
  reaches the leaf boxed through contract items C5 and C6 and FR-403's items
  7 and 8;
- `ProtocolClauseFrameRefusal`, which carries `model::observation`'s
  `AdmissionRecord` and is raised by frame check 11 before any Begin.

### What stays in `qsl-eval`

`qsl-eval` SHALL keep, as callers of the clause entry:

- the FR-107 clause selection and its input refusals (`UnknownClause`,
  `ObservationsMismatch`, `DuplicateClause`) before Begin, the frame built
  from the admitted observations, and the clause-set order, short-circuit
  and combination;
- frame check 11 (FR-115), which evaluates no checked term;
- the FR-366 value collection, pairing and ordering, one Begin per pair;
- the FR-265 trail recorder, the receiver path, the FR-265 derivation and
  the separation check's steps;
- the FR-380-AC-7 population-bound check;
- the clause environment contract implementation;
- the simulation, automaton, hyper, temporal, fairness and model-subject
  engines, which reach evaluation through the paths above;
- the routing of `CheckedPackageEvaluation::evaluate`: a checked expression
  that contains `Pre`, `NamedPopulation`, `PopulationObjects` or `History`
  begins the clause entry in expression form and the `Value` regime, over
  the package's `ObjectEnvironment` as the `own` side; any other checked
  expression begins the `Value`-family entry. Finding the node kinds is a
  structural walk that evaluates nothing.

Until FR-405's switch, the production paths run the reference, and the
clause entry runs through FR-403's separate `quire-eval` entry, which
`qsl-eval` extends with one clause route per path above. FR-405's switch
moves every path onto the clause entry or the `Value`-family entry in one
change and deletes the separate entry.

### Parity

The clause entry and the reference SHALL agree on every result, refusal,
location, loss, admitted charge, reported read and trail-derived record.
FR-402's version-1 case carries no object environment, no domain package
and no observations, so a clause input cannot be a corpus case. Parity is
checked in two ways:

1. **The `Value`-family corpus.** With the clause entry in `quire-eval`,
   FR-402's harness over the QSL-owned corpus reports zero divergences,
   and each case's compared record equals its record before the clause
   entry existed. The corpus checks that the clause entry leaves the
   `Value`-family entry unchanged; it does not exercise the clause entry.
2. **The clause tests.** Each clause-path test that the criteria below
   name SHALL pass against the reference and against the clause entry, and
   each asserts the reference's results and charges.

After FR-405's switch the reference is gone. Each test then runs once, on
`quire-eval`, with the reference's results as recorded expected values.

### Size on RT's target

The clause entry is generic over the clause environment and the recorder.
It is compiled into a binary only when the binary begins it.
`agent-ix/quire-eval`'s gate SHALL build two `#![no_std]` probe crates for
`thumbv7em-none-eabi` in release mode. They are not published:

- `probe-value` begins only the `Value`-family entry over the closure-only
  environment, as RT does;
- `probe-clause` also begins the clause entry, with a trivial clause
  environment and recorder.

The gate SHALL report the `.text` and `.rodata` sizes of both probes. The
change that adds the clause entry SHALL report both sizes at its base and
at its head in its description. `probe-value`'s combined `.text` and
`.rodata` at the head SHALL exceed the base by at most 2%. RT's own
footprint limits stay RT's (AD-001 "RT and CG acceptance"). No size is
recorded in a checked-in file.

## Acceptance Criteria

Each criterion below that compares the clause entry with the reference
holds as written until FR-405's switch. After the switch, the test runs once
on `quire-eval`, and the reference's results are its recorded expected
values.

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-406-AC-1 | `quire-eval`'s manifest still meets FR-403-AC-1 and declares no feature that adds or removes the clause entry. `quire-eval`'s source names no `qsl_` path. Its `thumbv7em-none-eabi` gate build passes with the clause entry. `quire-eval`'s closure-only environment implements only FR-403's contract. | Test and Inspection (TC-1180) |
| FR-406-AC-2 | With the clause entry in `quire-eval`, FR-402's harness reports zero divergences over the QSL-owned corpus, each case's compared record equals its record at the clause-entry change's base, and FR-403-AC-13's `Value`-family Begin over `pre(allInstances<T>(p))` still returns `CallFailure::Fault` with stage `quire-eval`, invariant `outside-value-family` and no charge. | Test (TC-1180) |
| FR-406-AC-3 | Over the FR-108 corpus, FR-107-AC-1 to AC-5 and AC-13 to AC-20 pass through the clause entry with the reference's outcomes, clause-set entries and outcomes, consumed counters and admitted charges. FR-107-AC-3's `reaches` charge log is the five charges it lists. Evaluating each corpus case ten times gives equal results and charge logs. | Test (TC-1180) |
| FR-406-AC-4 | `Pre`: FR-107-AC-2 passes through the clause entry, with `pre(self.versionNumber)` 2 and `self.versionNumber` 3. A postcondition that reads `pre(self.versionNumber)` over observations with no pre side returns `FamilyResult::Refused` with `wrong_snapshot`/`wrong-anchor`, located at the read, with the reference's charges. FR-405-AC-6's expression, through `CheckedPackageEvaluation::evaluate` with a pre anchor recording `a`, `b` and a post recording `a`, `b`, `c`, completes with `a` and `b` in ascending reference-key order, with the reference's charges, and leaves one `ObservedRead::Membership` with observation `Pre` in the `ReadLog`. Every test in `qsl-eval/tests/it/model_reference_queries.rs` that evaluates a `pre(...)` expression passes with the reference's expected values. | Test (TC-1180) |
| FR-406-AC-5 | `NamedPopulation` (FR-391): over a snapshot listing population `P` `complete` with members `m1`, `m2`, `m3`, two of them conforming to `T`, `allInstances<T>(M::P)` returns those two in ascending reference-key order, with three `population.visit` charges and one `collection.bound` sized 2. With `P` not listed `complete`, it returns `FamilyResult::Refused` with `incomplete_population`/`incomplete-scope` and field `population`. `lookup<T>(M::P, r) absent undefined` with an absent `r` returns the `state-model` undefined result with `binding` and `key` fields. `absent refused` returns the `ModelQueryRefusal` code. A non-conforming `r` returns the type-mismatch `ModelQueryRefusal` code. Each result, location and charge sequence equals the reference's. | Test (TC-1180) |
| FR-406-AC-6 | `PopulationObjects` and `History`: FR-139's refinement-row tests and FR-138's history-update tests pass through the clause entry with the reference's values and charges. A history field with no value at the read's observation returns `FamilyResult::Refused` with `missing_declaration`/`missing-name` and field `history` naming the field. A `PopulationObjects` read returns its members in ascending reference-key order, and the same result and charges on each of ten runs. | Test (TC-1180) |
| FR-406-AC-7 | `Clause` regime: a state clause that reads `self.versionNumber` admits `model.navigate` before the read, and, with `deref(...)`, `model.deref` before that. The same body in the `Value` regime admits neither. A state clause whose body is `allInstances<T>(p)` over a `Population` parameter returns `FamilyResult::Refused` with `unknown_required_feature`/`unsupported-feature`, located at the node. | Test (TC-1180) |
| FR-406-AC-8 | FR-265 trail: with `qsl-eval`'s recorder, FR-107-AC-7 to AC-12 and FR-265-AC-1 to AC-7 pass through the clause entry. The receiver paths, the read entries in order with their paths, the decisions, the stop reports, the bases and the separating witness records equal the reference's. | Test (TC-1180) |
| FR-406-AC-9 | ADR-031 SW-13: FR-268's separation-check tests pass through the clause-entry steps, with the reference's `Separation` result for each, the reference's admitted charges, and no Begin for the element step. | Test (TC-1180) |
| FR-406-AC-10 | Over FR-265-AC-1's `AllBelow` over `high`, a Begin with no recorder and a Begin with a recorder that accepts every event return equal evaluations, consumed counters, admitted charges and reported reads. A recorder that returns an `InternalFault` at the first `stopped` event ends the session with `CallFailure::Fault` carrying that fault, and the admitted charges stop at that event. | Test (TC-1180) |
| FR-406-AC-11 | Each of `evaluate_closed`, `evaluate_reward`, `evaluate_terminal_predicate`, `evaluate_refinement_row`, `evaluate_history_update`, `evaluate_state_predicate`, `evaluate_protocol_slot` and the FR-366 value-invariant evaluation has its tests pass through the clause entry in the regime this requirement assigns it, with the reference's expected values. | Test (TC-1180) |
| FR-406-AC-12 | Begin in body form over a node of an imported library's graph returns `CallFailure::Fault` with stage `quire-eval` and invariant `clause-root-outside-closure`, with no charge. Begin in expression form over `x + 1` with `x: Int[0, 9]` given `10` returns `CallFailure::Input` with no charge, and given two values returns `InputRefusal::Arity`, as FR-403-AC-12's `Value`-family Begin does. | Test (TC-1180) |
| FR-406-AC-13 | `HistoryUnresolved`, `NamedPopulationIncomplete`, `ProtocolClauseSnapshot` and `ProtocolClauseUnsupported` are defined in `quire-eval` and in no QSL crate. `qsl-eval` defines `ModelQueryRefusal` and `ProtocolClauseFrameRefusal`, and none of the four. Each cause's catalog code and fields equal the reference's for AC-4 to AC-7's results. | Test and Inspection (TC-1180) |
| FR-406-AC-14 | `agent-ix/quire-eval`'s gate builds `probe-value` and `probe-clause` for `thumbv7em-none-eabi` in release mode and reports each one's `.text` and `.rodata` sizes. The clause-entry change's description reports both at its base and its head, and `probe-value`'s combined `.text` and `.rodata` at the head is at most 102% of its base. | Test and Inspection (TC-1180) |

## Dependencies

- [FR-262](FR-262-evaluate-values-and-calls-at-any-depth.md): the session,
  scheduler, charges and depth behaviour the clause entry shares.
- [FR-403](FR-403-implement-the-no-std-evaluator-in-quire-eval.md): the
  `Value`-family entry, its argument admission, its environment contract
  and its separate `qsl-eval` entry.
- [FR-401](FR-401-host-the-evaluation-interface-in-the-quire-eval-leaf.md)
  and [FR-400](FR-400-separate-checked-terms-from-the-checker.md): the leaf,
  its dependency set, and the checked terms and `Observation` in C.
- [FR-107](FR-107-evaluate-state-clauses-at-s6a.md),
  [FR-265](FR-265-derive-a-state-clause-separating-witness.md),
  [FR-268](FR-268-check-a-state-clause-witness-on-replay.md) and
  [ADR-031](../decisions/ADR-031-state-forall-separating-witness.md): the
  state clause, its trail, the witness derivation and the separation check.
- [FR-106](FR-106-admit-snapshots-and-invocations.md), FR-115, FR-138,
  FR-139, FR-366, FR-380 and [FR-391](FR-391-name-a-population-in-a-state-clause.md):
  the observations and the clause reads.
- [FR-402](FR-402-differentially-test-the-no-std-evaluator-against-the-std-reference.md):
  the corpus and harness the `Value`-family parity runs on.
- [FR-405](FR-405-retire-the-std-reference-evaluator-for-quire-eval.md):
  the switch this entry is a prerequisite of (IR-593).
- [NFR-015](../non-functional/NFR-015-build-evaluator-crates-no-std-behind-a-default-std-feature.md):
  the `thumbv7em-none-eabi` target and toolchain.
- [AD-001](../assurance/AD-001-shared-evaluation-core-interface.md) and
  [ADR-011](../decisions/ADR-011-stage-dag-and-dependency-architecture.md)
  §6.1 layer E and §7.2.

## References

- Owning ticket: Linear STD-318. Blocks IR-593 (S9) and IR-584.
