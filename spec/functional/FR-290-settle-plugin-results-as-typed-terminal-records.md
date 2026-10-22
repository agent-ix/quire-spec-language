---
id: FR-290
title: "Settle plugin results as typed terminal records, with plugin proofs labelled trusted"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-030
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/ADR-029
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-288
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-289
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-098
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-298
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-281
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-282
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-127
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-314
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-062
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-070
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-071
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-278
    type: depends_on
  - target: "ix://agent-ix/quire-specification/FR-272"
    type: depends_on
  - target: "ix://agent-ix/quire-specification/FR-305"
    type: depends_on
  - target: "ix://agent-ix/quire-specification/FR-331"
    type: depends_on
  - target: "ix://agent-ix/quire-specification/FR-354"
    type: depends_on
  - target: "ix://agent-ix/quire-specification/FR-462"
    type: depends_on
---
# FR-290: Settle plugin results as typed terminal records, with plugin proofs labelled trusted

## Description

A plugin is either a compile-time Rust implementation of the driver's
`Provider` trait, linked into the binary, or a third-party executable that
runs as its own operating-system process and speaks the plugin wire
`quire.plugin-wire/v1` (ADR-029 PL-1 to PL-5). QSpec FR-305 owns the plugin
forms and QSpec FR-462 owns the wire. An installed plugin is a trusted
program, like any program the user installs, and runs with the user's own
authority.

This requirement states how a plugin's result becomes a QSL terminal record:

- Every result is read with the typed FR-331 reader. A plugin receives the
  package's v2 bytes and returns FR-331 results; it never produces a checked
  object, and IR's reader revalidates the `package_id` it was given
  (ADR-011 §4).
- A plugin's counterexample is canonical assignments. Its item settles
  `refuted` only when S6a replay reproduces it (FR-289 item 4).
- A plugin's `proved` result has no replay and no in-core certificate
  checker. Its terminal record keeps the result `proved`, names the plugin's
  `BackendId` (FR-288), and carries the certification `trusted`. A renderer
  shows the label beside the verdict (FR-298).

The terminal record type and its `Certification` member are QSL's, in
layer 6 `qsl-replay` (FR-127, ADR-018 PC-1). QSL settles the records of its
native engines and of process plugins there; CG keeps the C-09 map that
settles a Kani outcome (ADR-013 C-09), run by the orchestrating driver
(ADR-011 T-13). Every `proved` record carries exactly one certification, by
its producer:

| Producer of the `proved` result | Settled by | Certification | The record also names |
| --- | --- | --- | --- |
| An `analyze` engine whose certificate a qualified-core checker accepted (FR-281, FR-282) | QSL, `qsl-replay` | `certified` | the checker that accepted it |
| A native `analyze` engine with no qualified-core certificate checker (FR-281) | QSL, `qsl-replay` | `uncertified` | the engine |
| A third-party out-of-process plugin | QSL, `qsl-replay` | `trusted` | the plugin's `BackendId` |
| The Kani backend, whose prove path is part of the qualified core (ADR-029 CB-2) | CG, C-09 | `certified` | the backend's `BackendId` |
| The SMT provider route | QSL, `qsl-replay`, after FR-314's check (FR-127) | `certified` when FR-314 verifies the solver's certificate, `uncertified` when the certificate is absent or unverifiable | the backend's `BackendId` |

QSpec FR-331 owns the wire member `certification` and its three values;
QSpec FR-305 states the plugin row and QSpec FR-354 the SMT row.

### Reading a plugin result member

The driver holds a plugin's FR-331 result as wire JSON. `qsl-replay` gives it
four readers, re-exported at the crate root. Each takes one member as a
`serde_json::Value`, as `read_supplied_limits` and `read_object_environment`
do:

- `read_plugin_basis` reads a `proved` entry's `basis` into a `ProofBasis`.
- `read_plugin_certification` reads a `proved` entry's `certification`.
- `read_plugin_inconclusive_cause` reads an `inconclusive` entry's `cause`
  into an `InconclusiveCause`.
- `read_plugin_assignments` reads the item's `counterexamples` entry into the
  `Vec<CanonicalAssignment>` that `ReplaySource::Input` replays.

Each reader returns the typed value or a `PluginResultRefusal`. The readers
accept exactly the FR-331 wire spellings, and they refuse every value a
plugin may not send (ADR-029 PL-7). A refused member settles its item
`failed`, naming the plugin's `BackendId`, as every refused plugin result
does (FR-289). Each reader checks a member in this order and returns the
first refusal:

1. The member is not a JSON object: `Malformed` at the member.
2. The member has no `type`: `MissingMember` at `/<member>/type`. A `type`
   that is not a string: `Malformed` there.
3. The `type` is a QSL-only cause (`replay-parity`, `replay-refused`,
   `kani_vacuous_proof`): `QslOnlyCause`. The `type` is a core-only basis
   (`bounded-proof`, `closed-form`, `witnessed`, `exact-value`,
   `value-bounds`, `zone-certified`): `CoreOnlyBasis`. This step reads the
   spelling only, so neither refusal depends on the payload.
4. The `type` is not in the FR-331 closed set for the member:
   `UnknownSpelling`. Every spelling outside the FR-331 set is unknown. That
   includes `kani-vacuous-proof` and QSL's own outcome-document spellings
   that FR-331 does not list, such as `scalar-agrees` and
   `state-constraint`.
5. The `type` is an FR-331 value that QSL's type cannot hold for a plugin:
   `Unsupported`. The value is valid FR-331, but QSL cannot settle it.
6. A required payload member is absent: `MissingMember` at its pointer. When
   several are absent, the reader names the first in member-name order.
7. A member the spelling does not define is present: `UnknownMember` at its
   pointer, again the first in member-name order.
8. A payload member has the wrong JSON kind or a value outside its domain:
   `Malformed` at its pointer.

A pointer is an RFC 6901 JSON pointer. For `basis`, `certification` and
`cause` it is relative to the result entry, as in `/basis/depth`. For
assignments it is relative to the `counterexamples` entry, as in
`/counterexample/values/0/value`. An FR-331 count is a JSON integer from 0 to
2^53 − 1. A positive count starts at 1.

| Wire `basis` | `ProofBasis` | Payload rules |
| --- | --- | --- |
| `{"type": "exhaustive"}` | `Exhaustive` | no payload member |
| `{"type": "bounded-complete", "depth": d}` | `BoundedComplete { depth: d }` | `depth` a count |
| `{"type": "inductive", "depth": d}` | `Inductive { depth: d }` | `depth` a count |
| `{"type": "bounded-solver", "method": m, "horizon": h, "jumps": j}` | `BoundedSolver { method, horizon, jumps }` | `method` is a `SolverMethod` spelling: `delta-complete`, `flowpipe-outward-rounded` or `flowpipe-unrounded`. Any other non-empty string is `Unsupported` at `/basis/method`, because QSL decides soundness from the method kind (FR-250). `horizon` is a checked-package v2 `Rational` `{numerator, denominator}` in lowest terms, with a non-negative numerator (`SolverHorizon`) and a positive denominator. `jumps` is a count. |
| `{"type": "reduced", "reductions": [...]}` | `Reduced { reductions }` | `reductions` is a non-empty array, kept in wire order, of `{reduction, parameters, row}`. `partial-order` with `parameters` `{"proviso": "breadth-first-revisit"}` reads as `AppliedReduction::PartialOrder { proviso: BreadthFirstRevisit }`. `copy-swap` with `parameters` `{"classes": [...]}`, each class two or more distinct non-empty strings, reads as `AppliedReduction::CopySwap { classes }`. `row` is a non-empty string; the reader checks it and does not keep it. `symmetry` is `Unsupported` at `/basis/reductions/<i>/reduction`: its wire parameters are FR-382 symmetry declarations, not the group, orbit and stabiliser that `AppliedReduction::Symmetry` holds. |

A plugin `proved` entry's `certification` is the string `trusted`
(QSpec FR-331 "Certification"). An absent `certification` is `MissingMember`
at `/certification`. `certified`, `uncertified` and every other value are
`UnknownSpelling` for the member `certification`. The record carries `trusted`
whatever the entry says, so the reader returns no value. This step runs before
`read_plugin_basis`.

| Wire inconclusive `cause` | `InconclusiveCause` | Payload rules |
| --- | --- | --- |
| `undecided-successor`, `no-initial-state`, `no-instance`, `sufficient-test-failed`, `solver-inconclusive`, `match-undetermined`, `vacuous-match` | the variant of the same name | no payload member |
| `bound-reached{depth}`, `induction-not-closed{depth}` | `BoundReached { depth }`, `InductionNotClosed { depth }` | `depth` a count |
| `reduction-not-preserving{reduction, form}` | `ReductionNotPreserving { reduction, form }` | `reduction` is `symmetry`, `partial-order` or `constraint`, read as `ReductionKind::Symmetry`, `PartialOrder` or `StateConstraint`. `form` is a `PropertyForm` canonical spelling (quire-semantic-value FR-121). Any other string is `Malformed` at its pointer. |
| `constraint-reached{boundary_states}` | `ConstraintReached { boundary_states }` | a positive count |
| `reduction-horizon{max_depth}` | `ReductionHorizon { max_depth }` | a count |
| `memory-bound-reached{limit, value, states}` | `MemoryBoundReached { bound: MemoryBound { member, value }, states }` | `limit` is `max_store_buffer` or `max_messages`; `value` and `states` are positive counts |
| `instance-bound-reached{limit, value, states}` | `InstanceBoundReached { bound: InstanceBound { value }, states }` | `limit` is `max_live_instances`; `value` and `states` are positive counts |
| `mapping-undetermined{row, state_key, prefix}` | `MappingUndetermined` | `row` is a non-empty string, `state_key` a string and `prefix` an array. The reader checks them and does not keep them, because the variant carries no payload. |

The reader returns `Unsupported` for these FR-331 inconclusive causes:

- `certificate-rejected`: FR-331-AC-26 binds a rejection to the checker
  that the request's claim resolves. A plugin proof has no in-core checker
  (ADR-029 PL-7), so there is no checker to bind it to.
- `well-definedness-unchecked`: the QSL variant records witnesses that QSL
  replayed (FR-169), and a plugin's witnesses are not replayed.
- `symmetry-broken`: its wire payload does not carry the `DeclarationKey`
  and permutation that the variant holds.
- `lasso-not-concretized`, `no-admitted-behaviour`,
  `transfer-not-preserving`, `premise-unproved` and
  `hypotheses-not-established`: `InconclusiveCause` has no variant for them.

`read_plugin_assignments` reads a `counterexamples` entry. An entry that names
a `transcript`, an entry with no `counterexample`, and an entry whose
`counterexample` `type` is not `assignments` each have nothing to replay as
canonical assignments. Each is refused `MissingCounterexample` (ADR-029 PL-7),
before any other check. In an `assignments` counterexample, `values` is an
array of `{parameter, value}`:

- `parameter` is an FR-331 `NodeId`, 64 lowercase hex digits, read as a
  `WireNodeId`.
- `value` is an FR-181 typed value. The reader reads its RFC 8785 encoding
  with FR-070's witness-value-text decode into a `WitnessValue`. A value that
  decode refuses is `Malformed` at `/counterexample/values/<i>/value`.
- A parameter repeated in `values` is `Malformed` at the later entry's
  `/parameter`.
- An empty `values` is a call with no arguments.

The declared type of each value is checked later, when `replay` converts the
argument (FR-098).

`PluginResultRefusal::diagnostic(item_index, received)` builds the
outcome-document diagnostic (`OutcomeDiagnostic::new`, ADR-029 CB-4). Its
detail names the result item index, the member or pointer, and the received
spelling or JSON kind. Every code comes from QSpec FR-272's closed table:

| Refusal | Code / cause |
| --- | --- |
| `QslOnlyCause`, `CoreOnlyBasis`, `UnknownSpelling` | `invalid_result`/`unknown-cause` (member `cause`, `basis` or `certification`) |
| `MissingMember` | `invalid_result`/`missing-member` |
| `MissingCounterexample` | `invalid_result`/`missing-member` (member `counterexample`) |
| `UnknownMember` | `unknown_member`, with `document_pointer` |
| `Malformed` | `malformed_wire`, with `document_pointer` |
| `Unsupported` | `unknown_required_feature`/`unsupported-feature`: the feature is the received spelling, and the consumer is plugin settlement |

`read_plugin_cause_kind` keeps its narrower contract (FR-289-AC-11,
FR-289-AC-13). `read_plugin_inconclusive_cause` applies it at step 3.
`TerminalRecord::from_plugin_proof` and
`TerminalRecord::from_plugin_inconclusive` stay the settlement steps. They
take the values these readers return.

### Replay context

A plugin counterexample settles only through its S6a replay (FR-289 item 4).
The driver needs a `ReplayRequestWire` for that replay, and every member must
come from something QSL produced. The driver supplies no member of its own.
`PluginReplayContext`, in `qsl-replay` and re-exported at the crate root,
holds the package-wide members. `PluginReplayContext::new` takes:

- the `CheckedUnit` from `check`;
- the `EmittedUnit` that `package` emitted from it;
- the `AdmittedModels` from `select`;
- the run's `CallerLimits` (FR-255), whose `spine` limits the compile ran
  under (FR-278).

It copies what it needs and refuses nothing. A pair of units that do not
match is caught by the replay's own recompile, which requires the recompiled
`package_id` to equal the request's (FR-098).

`PluginReplayContext::request(occurrence, proof_bounds, plugin, assignments)`
builds one counterexample's `ReplayRequestWire`. It takes:

- the routed item's `OccurrenceKey` and the substituted `ProofBound`s of its
  extent, the values the driver wrote into the FR-331 request item (QSpec
  FR-331 "Items");
- the plugin's `BackendId`;
- the assignments from `read_plugin_assignments`.

| `ReplayRequestWire` member | Value | Source |
| --- | --- | --- |
| `profile_selections` | the semantic profile selections of the checked package's lock | ADR-013 O-25, FR-071 |
| `package_id` | the emitted package's `package_id`, in `quire.package.semantic/v2` | FR-071, FR-098 |
| `source_digests` | the unit's own source reference, the one source the recompile reads | FR-071, FR-098 |
| `dependencies` | one entry per `dependency_selections` entry of the lock, in order: its `identity`, its `package_id`, and as `sources` the reference of the library source that `check` resolved for it (`CheckedUnit::libraries`) | FR-071, ADR-015 D-4 |
| `selected_function` | the owner of the item's requirement record (FR-062 "Owner"), as a `QualifiedName` | FR-062, FR-098 |
| `source` | `ReplaySource::Input(assignments)` | ADR-029 PL-7, ADR-013 O-25 |
| `obligation_identity` | the `process-provider` obligation identity below | ADR-013 O-09 |
| `backend` | the plugin's `BackendId` string | ADR-013 O-19, ADR-029 PL-5 |
| `state_environment` | empty: the FR-331 request carries no state environment, so the plugin run started from none | FR-071 |
| `accounting_limits` | the run's accounting budgets, `CallerLimits::accounting` | FR-071, FR-255 |
| `stage_limits` | every FR-255 setting that `CallerLimits::spine` owns, with its bound (`SettingLimits::bounds`), so the replay compiles under the limits the package was compiled under and never under QSL's defaults | FR-071, FR-263, FR-399 |
| `supplied_limits` | `None`, which means the published defaults | FR-071 |
| `object_environment` | `None`: canonical assignments carry no snapshot | FR-071, FR-098 |
| `declared_domains` | `DeclaredDomain::new(bound)` for each item proof bound, in the item's order | FR-071, ADR-014 B-4 |
| `byte_provision` | each source of `EmittedUnit::sources()` under its raw-byte digest, and each domain package document that `AdmittedModels` holds under its `sha256-jcs` digest | FR-071, FR-278 |

The `process-provider` obligation identity is the SHA-256 of the RFC 8785
encoding, through ADR-013 §2's one encoder, of an object with these members:

- `declared_domains`: the item's proof bounds, in O-09's parity `bounds` form
  (`{"tag": "bounds", "entries": [{key, bound}]}`, with entries ascending by
  the encoded `key`);
- `node`: the hex of the occurrence key's node;
- `obligation_kind`: the record's FR-290 capability kind spelling, such as
  `value-validity`;
- `occurrence_key`: `{ordinal, role}`;
- `subject_kind`: the string `process-provider`.

A process item has no CG obligation (ADR-029 PV-4 ruling 3), so QSL computes
this identity itself. The `subject_kind` tag keeps it distinct from every CG
obligation form. The encoding rules are O-09's parity rules.

`request` checks in this order and returns the first `PluginReplayRefusal`:

1. The occurrence key names no requirement record of the checked package:
   `UnknownOccurrence`, `missing_declaration`/`missing-name`, retaining the
   key.
2. The record has no owner (an `operation-contract` or
   `temporal-satisfaction` record, FR-062): `NoReplayFunction`,
   `unknown_required_feature`/`unsupported-feature`, retaining the record's
   kind. No function takes the assignments.
3. The encoder refuses the identity preimage (a JSON integer above 2^53):
   `Identity`, `noncanonical_wire`/`inexact-integer`, retaining the
   encoder's error.

Each refusal settles the item `failed`, naming the plugin's `BackendId`. Its
diagnostic goes in the outcome document, as for `PluginResultRefusal`.
`request` does not decode or verify the request it builds. `replay` reads it
through `ReplayRequest::decode` (FR-071). Every FR-071 and FR-098 refusal
that follows settles through `TerminalRecord::from_plugin_counterexample`
(FR-289).

## Inputs

An FR-331 result from a process provider, the items routed to it, and the
package's byte provision for replay. For a counterexample, the inputs also
include the `CheckedUnit`, `EmittedUnit` and `AdmittedModels` of the
package's compile and the run's `CallerLimits`.

## Outputs

One QSL terminal record per routed item.

## Behavior

- QSL shall read every process-provider result with the typed FR-331 reader
  before it builds a terminal record from it.
- When a process provider reports a counterexample, QSL shall replay it
  through layer-6 `replay` (FR-098) and settle the item `refuted` only if the
  replay reproduces it.
- If the replay does not reproduce the counterexample, then QSL shall settle
  the item `inconclusive` with cause `replay_parity`, or with cause
  `replay_refused` and the refusal's catalog code when the replay refuses.
- When a process provider reports `proved`, QSL shall settle the item
  `proved` with the provider's `BackendId` and the certification `trusted`.
- When a qualified-core certificate checker accepts an `analyze` engine's
  certificate, QSL shall settle the item `proved` with the certification
  `certified` and the checker's name.
- When an `analyze` engine with no qualified-core certificate checker
  reports a proof, QSL shall settle the item `proved` with the
  certification `uncertified` and the engine's name.
- The terminal record type shall hold a Kani `proved` that CG's C-09 map
  settles with the certification `certified`.
- `read_plugin_basis`, `read_plugin_certification`,
  `read_plugin_inconclusive_cause` and `read_plugin_assignments` shall accept
  exactly the wire values that the tables in "Reading a plugin result member"
  list.
- Each reader shall refuse a member it does not accept, in the order steps 1
  to 8 give, with the `PluginResultRefusal` and the FR-272 code that the
  refusal table gives.
- A reader shall refuse a QSL-only cause or a core-only basis by its
  spelling alone, whatever its payload.
- `PluginReplayContext::request` shall build each `ReplayRequestWire` member
  from the source that the replay-context table gives. It shall take no other
  input.
- When the occurrence names no requirement record, or the record has no
  owner, or the identity preimage does not encode, `request` shall refuse in
  that order and return no request.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-290-AC-1 | A process-provider FR-331 result of `proved` for a routed item settles a terminal record with value `proved`, the provider's `BackendId` and certification `trusted`; the FR-281-AC-1 record's certification is `certified`, naming the EN-5 checker; FR-281-AC-4's record from the test engine with no checker has certification `uncertified`, naming that engine; a Kani `proved` record built as C-09 settles it carries `certified`. | Test (TC-774) |
| FR-290-AC-2 | A process-provider counterexample that replay reproduces settles `refuted`; one whose assignment replay evaluates to `true` settles `inconclusive` with cause `replay_parity`; one naming a parameter the function lacks settles `inconclusive` with cause `replay_refused`. | Test (TC-774) |
| FR-290-AC-3 | A process-provider result that fails the FR-331 reader builds no terminal record from its body, and its items settle as FR-291 states. | Test (TC-774) |
| FR-290-AC-4 | The text rendering (FR-298) of AC-1's plugin record shows `proved` with the label `trusted` beside it, and its JSON outcome document holds the certification `trusted`; the `uncertified` record renders with the label `uncertified` beside `proved`; the AC-1 Kani record renders `proved` with the label `certified` beside it. | Test (TC-774) |
| FR-290-AC-5 | `read_plugin_basis` reads `{"type":"exhaustive"}` as `Exhaustive`, `{"type":"bounded-complete","depth":4}` as `BoundedComplete{depth:4}`, `{"type":"inductive","depth":0}` as `Inductive{depth:0}`, `{"type":"bounded-solver","horizon":{"denominator":"2","numerator":"5"},"jumps":3,"method":"delta-complete"}` as `BoundedSolver{DeltaComplete, 5/2, 3}`, and `{"type":"reduced","reductions":[{"parameters":{"proviso":"breadth-first-revisit"},"reduction":"partial-order","row":"invariant"},{"parameters":{"classes":[["a","b"]]},"reduction":"copy-swap","row":"hyper"}]}` as `Reduced` holding `PartialOrder{BreadthFirstRevisit}` then `CopySwap{[["a","b"]]}`; each read value passed to `TerminalRecord::from_plugin_proof` settles `proved`, `trusted`, naming the plugin. | Test (TC-774) |
| FR-290-AC-6 | `read_plugin_basis` refuses, each with its refusal and pointer: each of the six core-only spellings carrying its valid FR-331 payload (`CoreOnlyBasis`), and `{"type":"closed-form","extra":1}` (`CoreOnlyBasis`, the spelling read before members); `{"type":"bounded_complete","depth":4}` (`UnknownSpelling`); `"exhaustive"` (`Malformed` at `/basis`); `{"depth":4}` (`MissingMember` at `/basis/type`); `{"type":"inductive"}` (`MissingMember` at `/basis/depth`); `{"type":"exhaustive","depth":3}` (`UnknownMember` at `/basis/depth`); `{"type":"inductive","depth":"3"}`, `{"type":"inductive","depth":-1}` and `{"type":"inductive","depth":9007199254740992}` (`Malformed` at `/basis/depth`); a `bounded-solver` with `method` `"dreal"` (`Unsupported` at `/basis/method`), with horizon `{"denominator":"1","numerator":"-1"}` or `{"denominator":"4","numerator":"2"}` (`Malformed` at `/basis/horizon`); a `reduced` whose first reduction is `symmetry` with one valid FR-382 declaration (`Unsupported` at `/basis/reductions/0/reduction`); and a `copy-swap` class `["a","a"]` (`Malformed`). | Test (TC-774) |
| FR-290-AC-7 | `read_plugin_certification` accepts `"trusted"`; it refuses `"certified"` and `"uncertified"` with `UnknownSpelling` for the member `certification`, and an entry with no `certification` with `MissingMember` at `/certification`. | Test (TC-774) |
| FR-290-AC-8 | `read_plugin_inconclusive_cause` reads each payload-free spelling of its table as its variant, `{"type":"bound-reached","depth":7}` as `BoundReached{7}`, `{"type":"reduction-not-preserving","form":"liveness","reduction":"constraint"}` as `ReductionNotPreserving{StateConstraint, Liveness}`, `{"type":"constraint-reached","boundary_states":2}`, `{"type":"reduction-horizon","max_depth":5}`, `{"type":"memory-bound-reached","limit":"max_messages","states":3,"value":2}` as `MemoryBoundReached{MaxMessages 2, 3}`, `{"type":"instance-bound-reached","limit":"max_live_instances","states":1,"value":4}`, and a `mapping-undetermined` with `row`, `state_key` and an empty `prefix` as `MappingUndetermined`; each read value passed to `TerminalRecord::from_plugin_inconclusive` settles `inconclusive` with it. | Test (TC-774) |
| FR-290-AC-9 | `read_plugin_inconclusive_cause` refuses `replay-parity`, `replay-refused` and `kani_vacuous_proof`, each also with an extra member, with `QslOnlyCause`; `kani-vacuous-proof`, `scalar-agrees` and `bound_reached` with `UnknownSpelling`; each of the eight causes of its `Unsupported` list, each with its valid FR-331 payload, with `Unsupported`; `{"type":"constraint-reached","boundary_states":0}` and `{"type":"memory-bound-reached","limit":"max_store","states":1,"value":1}` with `Malformed` at the member; and `{"type":"no-instance","depth":1}` with `UnknownMember` at `/cause/depth`. | Test (TC-774) |
| FR-290-AC-10 | `read_plugin_assignments` reads `{"backend":"acme","counterexample":{"type":"assignments","values":[{"parameter":"<64 hex>","value":{"type":"integer","value":"0"}}]},"request_index":0}` as one `CanonicalAssignment` of that `WireNodeId` and `ExactInteger(0)`, and an empty `values` as no assignment; it refuses an entry naming `transcript`, an entry with no `counterexample`, and one whose `counterexample` is a `TemporalCounterexample` with `MissingCounterexample`; a `parameter` of 63 hex digits or with an uppercase digit with `Malformed` at `/counterexample/values/0/parameter`; a value `{"type":"integer","value":"07"}` with `Malformed` at `/counterexample/values/0/value`; and a second entry repeating the first's parameter with `Malformed` at `/counterexample/values/1/parameter`. | Test (TC-774) |
| FR-290-AC-11 | The diagnostic of each refusal kind of AC-6 to AC-10 carries the code and cause of the refusal table, and its detail names the result item index, the member or pointer, and the received spelling or JSON kind. | Test (TC-774) |
| FR-290-AC-12 | For a unit compiled through `parse`, `select`, `check` and `package` whose Boolean function `p(x: Int[0, 9])` returns `x + 1 > 3`, the context built from its units and limits gives, for the value-validity item of `x + 1` with assignment `x = 0`, a `ReplayRequestWire` whose members are those of the replay-context table: `source_digests` the unit's one source, `selected_function` `p`, `backend` the plugin's identity, `declared_domains` the item's proof bounds, an empty state environment, no object environment, and a byte provision holding the unit's source. `replay` of it settles the input arm reproduced with category violation, and `TerminalRecord::from_plugin_counterexample` settles `refuted` naming the plugin. | Test (TC-774) |
| FR-290-AC-13 | The `process-provider` preimage for node `11` repeated 32 times, occurrence `{"ordinal":0,"role":"expression"}`, kind `value-validity` and one bound `integer_range` 0 to 9 at `DomainKey::Node` `22` repeated 32 times with an empty path encodes to `{"declared_domains":{"entries":[{"bound":{"lower":"0","tag":"integer_range","upper":"9"},"key":{"node_id":"2222222222222222222222222222222222222222222222222222222222222222","path":[],"tag":"node"}}],"tag":"bounds"},"node":"1111111111111111111111111111111111111111111111111111111111111111","obligation_kind":"value-validity","occurrence_key":{"ordinal":0,"role":"expression"},"subject_kind":"process-provider"}`, whose SHA-256 is `18ef6c73b25d98e73ae37c5af66b71618d9f7c081431b9cf148ff5ea37eda2c3`. With no bound the digest is `9d60d63dfd95ad827bfc17cd15c3941536dbb130c4fc17460d0e5170db19ec14`. AC-12's request carries the identity of its item's preimage. | Test (TC-774) |
| FR-290-AC-14 | The context built with `CallerLimits` whose `s1.tokens` is 200000 and whose accounting `work_units` is 5000 gives a request whose `stage_limits` holds `s1.tokens` 200000 and every other spine setting at its bound, and whose `accounting_limits` has `work_units` 5000. `request` refuses an occurrence key no record of the package has with `UnknownOccurrence` (`missing_declaration`/`missing-name`), and the `operation-contract` record of a state clause with `NoReplayFunction` (`unknown_required_feature`/`unsupported-feature`); an unknown occurrence is refused first. | Test (TC-774) |

## Dependencies

- ADR-029 PL-1, PL-4, PL-5, PL-7: the plugin forms, the result rules and the
  trusted label; rulings RU-4.
- ADR-011 §4, FB-07: checked objects and replay.
- ADR-013 O-16, O-24: categories and terminal records.
- [FR-288](FR-288-build-the-registry-from-provider-manifests.md): `BackendId`.
- [FR-289](FR-289-keep-results-unchanged-by-provider-installation.md): the meaning invariant.
- [FR-098](FR-098-execute-a-replay-request.md): replay.
- [FR-298](FR-298-render-outcomes-and-views-from-typed-values.md): rendering.
- [FR-062](FR-062-implement-checked-family-contract.md): requirement records and their owner.
- [FR-070](FR-070-implement-typed-counterexample-witness-envelope.md): witness value text.
- [FR-071](FR-071-implement-typed-replay-request.md): the replay request the context builds.
- [FR-278](FR-278-parse-select-check-and-package-as-library-operations.md): the units the context is built from.
- ADR-013 O-09, O-19, O-25: obligation identity, backend identity, the packet members.
- QSpec FR-305: the plugin forms; QSpec FR-462: the `quire.plugin-wire/v1`
  wire; QSpec FR-331: results, their wire spellings and the `certification`
  member; QSpec FR-354: the SMT route's certification; QSpec FR-272: the
  refusal codes.

## Overlap

The driver repository owns the plugin host, `quire-plugin-host`: starting
the process, reading frames and adapting a plugin to the `Provider` trait.

## References

- QSL-393 (V1-A06a): the plugin wire.
- QSL-390 (ARCH-50): owner ruling 4 and RU-4, recorded on the ticket.
- QSpec FR-305, FR-462, FR-354 (STD-141): the QSpec half.
- QSL-390: ruling RU-6 (`uncertified`), recorded on the ticket.
