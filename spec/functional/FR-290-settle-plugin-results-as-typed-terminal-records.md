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
  - target: ix://agent-ix/quire-spec-language/FR-057
    type: references
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
- `read_plugin_certification` reads a `proved` entry's `certification`. It
  takes an `Option<&serde_json::Value>`, so it can report an absent member.
- `read_plugin_inconclusive_cause` reads an `inconclusive` entry's `cause`
  into an `InconclusiveCause`.
- `read_plugin_assignments` reads the item's `counterexamples` entry into the
  `Vec<CanonicalAssignment>` that `ReplaySource::Input` replays.

Each reader returns the typed value or a `PluginResultRefusal`. The readers
accept only the FR-331 wire spellings, and refuse every value a plugin may
not send (ADR-029 PL-7). A refused member settles its item `failed`, naming
the plugin's `BackendId`, as every refused plugin result does (FR-289). For a
`proved` entry, the driver calls `read_plugin_certification` before
`read_plugin_basis`, so an entry that both readers would refuse is refused for
its `certification`.

A pointer is an RFC 6901 JSON pointer. For `basis`, `certification` and
`cause` it is relative to the result entry, as in `/basis/depth`. For
assignments it is relative to the `counterexamples` entry, as in
`/counterexample/values/0/value`; the empty pointer `""` names the whole
entry. An FR-331 count is a JSON integer from 0 to 2^53 − 1. A positive count
is at least 1. "Member-name order" is the order of member names by their
UTF-16 code units, the RFC 8785 order.

#### Basis and cause

`read_plugin_basis` and `read_plugin_inconclusive_cause` check their member in
this order, and return the first refusal:

1. The member is not a JSON object: `Malformed` at the member.
2. The member has no `type`: `MissingMember` at `/<member>/type`. If `type`
   is not a string: `Malformed` there.
3. The spelling is refused for a plugin whatever the payload. For a
   `basis`, a core-only basis (`bounded-proof`, `closed-form`, `witnessed`,
   `exact-value`, `value-bounds`, `zone-certified`) is `CoreOnlyBasis`. For a
   `cause`, a QSL-only cause (`replay-parity`, `replay-refused`,
   `stale-dependency`, `kani_vacuous_proof`) is `QslOnlyCause`. Each is at `/<member>/type`.
   Each check applies only to its
   own member. A `basis` that spells a QSL-only cause, or a `cause` that
   spells a core-only basis, is caught at step 4.
4. The `type` is not in FR-331's closed set for this member:
   `UnknownSpelling` at `/<member>/type`. That covers every spelling outside FR-331, including
   `kani-vacuous-proof`. It also covers QSL's own outcome-document
   spellings that FR-331 does not list, such as `scalar-agrees` and
   `state-constraint`.
5. The `type` is a valid FR-331 value that QSL's type cannot hold for a
   plugin: `Unsupported`.
6. A required payload member is absent: `MissingMember` at its pointer, the
   first absent one in member-name order.
7. A member the spelling does not define is present: `UnknownMember` at its
   pointer, the first such member in member-name order.
8. A payload member is outside its FR-331 domain: `Malformed` at its pointer,
   the payload members read in member-name order. This covers a wrong JSON
   kind and an out-of-range number. A payload
   string inside FR-331's domain that names something QSL's type cannot
   hold is `Unsupported` at its pointer, for the same reason as step 5. The
   two such strings are `basis` `method` and `cause` `form`.

| Wire `basis` | `ProofBasis` | Payload rules |
| --- | --- | --- |
| `{"type": "exhaustive"}` | `Exhaustive` | no payload member |
| `{"type": "bounded-complete", "depth": d}` | `BoundedComplete { depth: d }` | `depth` is a count |
| `{"type": "inductive", "depth": d}` | `Inductive { depth: d }` | `depth` is a count |
| `{"type": "bounded-solver", "method": m, "horizon": h, "jumps": j}` | `BoundedSolver { method, horizon, jumps }` | `method` is a non-empty string; an empty or non-string `method` is `Malformed`. The three `SolverMethod` spellings `delta-complete`, `flowpipe-outward-rounded` and `flowpipe-unrounded` read as their variant. Any other non-empty string is `Unsupported` at `/basis/method`, because `SolverMethod` cannot hold it. All three methods are accepted, `flowpipe-unrounded` included. A plugin's `proved` is trusted (ADR-029 PL-7, RU-4). FR-250's rule that a method kind decides soundness applies to the solvers QSL runs, not to a plugin's report. `horizon` is a checked-package v2 `Rational` `{numerator, denominator}` in lowest terms, with a non-negative numerator (`SolverHorizon`) and a positive denominator. `jumps` is a count. |
| `{"type": "reduced", "reductions": [...]}` | `Reduced { reductions }` | `reductions` is a non-empty array, kept in wire order, of `{reduction, parameters, row}`. `partial-order` with `parameters` `{"proviso": "breadth-first-revisit"}` reads as `AppliedReduction::PartialOrder { proviso: BreadthFirstRevisit }`. `copy-swap` with `parameters` `{"classes": [...]}` reads as `AppliedReduction::CopySwap { classes }`; each class holds two or more distinct non-empty strings, and a class that breaks this is `Malformed` at `/basis/reductions/<i>/parameters/classes/<k>`. `row` is a non-empty string. The reader checks `row` but does not keep it, because `AppliedReduction` has no member for it. `symmetry` is `Unsupported` at `/basis/reductions/<i>/reduction`: its wire parameters are FR-382 symmetry declarations, not the group, orbit and stabiliser that `AppliedReduction::Symmetry` holds. |

| Wire inconclusive `cause` | `InconclusiveCause` | Payload rules |
| --- | --- | --- |
| `undecided-successor`, `no-initial-state`, `no-instance`, `sufficient-test-failed`, `solver-inconclusive`, `match-undetermined`, `vacuous-match` | the variant of the same name | no payload member |
| `bound-reached{depth}`, `induction-not-closed{depth}` | `BoundReached { depth }`, `InductionNotClosed { depth }` | `depth` is a count |
| `reduction-not-preserving{reduction, form}` | `ReductionNotPreserving { reduction, form }` | `reduction` is `symmetry`, `partial-order` or `constraint`, read as `ReductionKind::Symmetry`, `PartialOrder` or `StateConstraint`; any other value is `Malformed`. `form` is a non-empty string, and an empty or non-string `form` is `Malformed`. A `PropertyForm` canonical spelling (quire-semantic-value FR-121) reads as that form. Any other non-empty string is `Unsupported` at `/cause/form`, as an unknown `method` is. |
| `constraint-reached{boundary_states}` | `ConstraintReached { boundary_states }` | `boundary_states` is a positive count |
| `reduction-horizon{max_depth}` | `ReductionHorizon { max_depth }` | `max_depth` is a count |
| `memory-bound-reached{limit, value, states}` | `MemoryBoundReached { bound: MemoryBound { member, value }, states }` | `limit` is `max_store_buffer` or `max_messages`; `value` and `states` are positive counts |
| `instance-bound-reached{limit, value, states}` | `InstanceBoundReached { bound: InstanceBound { value }, states }` | `limit` is `max_live_instances`; `value` and `states` are positive counts |
| `mapping-undetermined{row, state_key, prefix}` | `MappingUndetermined` | Each member is checked to its FR-331 domain: `row` a non-empty string, `state_key` an FR-331 `StateKey` (64 lowercase hex digits), and `prefix` an array of FR-331 `ModelStep` values (FR-364: `transition` and `post_state`, plus `delay` when present). The reader checks them but does not keep them, because the variant carries no payload. |

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

#### Certification

`read_plugin_certification` accepts exactly the JSON string `trusted` (QSpec
FR-331 "Certification", FR-331-AC-23: a plugin proof is `trusted`). It returns
no value, because an accepted entry's record carries `trusted`
(`TerminalRecord::from_plugin_proof`). It checks in this order:

1. The member is absent: `MissingMember` at `/certification`.
2. The member is not a string: `Malformed` at `/certification`.
3. The string is not one of FR-331's three values `certified`,
   `uncertified` and `trusted`: `Malformed` at `/certification`.
4. The string is `certified` or `uncertified`, which only the core and
   first-party engines may claim: `CertificationNotTrusted` at
   `/certification`, retaining the received string.

#### Assignments

`read_plugin_assignments` checks a `counterexamples` entry in this order,
which is QSpec FR-331's reading order ("Result entries"):

1. The entry is not a JSON object: `Malformed` at `""`.
2. The entry has no `counterexample`, including an entry that names a
   `transcript` instead: `MissingCounterexample` at `/counterexample`
   (ADR-029 PL-7: a plugin's counterexample is canonical assignments).
3. A member that FR-331 does not define for a plugin's entry is present:
   `UnknownMember`. At the entry, that is every member but `request_index`,
   `backend` and `counterexample`, so a `transcript` beside a
   `counterexample` is one. The reader names the first such member in
   member-name order.
4. The `counterexample` is not canonical assignments: `MissingCounterexample`
   at `/counterexample` when it is not an object, and at
   `/counterexample/type` when it has no `type`, its `type` is not a string,
   or its `type` is a string other than `assignments`, such as a
   `TemporalCounterexample`, a `ProbabilisticCounterexample`, graph evidence
   or a hyperproperty counterexample.
5. `values` is absent: `MissingMember` at `/counterexample/values`.
6. A member of the counterexample other than `type` and `values` is
   present: `UnknownMember`, the first in member-name order.
7. `values` is not an array: `Malformed` at `/counterexample/values`.
8. Each element `i`, in array order, is checked in turn:
   - an element that is not an object is `Malformed` at
     `/counterexample/values/<i>`;
   - an absent `parameter` or `value` is `MissingMember`, `parameter`
     first;
   - any other member is `UnknownMember`;
   - a `parameter` that is not an FR-331 `NodeId` (64 lowercase hex digits),
     or that equals an earlier element's, is `Malformed` at
     `/counterexample/values/<i>/parameter`;
   - a `value` that the FR-070 witness-value-text decode refuses, when it
     reads the value's RFC 8785 encoding as an FR-181 typed value, is
     `Malformed` at `/counterexample/values/<i>/value`.

An empty `values` is a call with no arguments. The reader reads each
`parameter` as a `WireNodeId` and each `value` as a `WitnessValue`. It does
not check the declared type of a value; `replay` checks it when it converts
the argument (FR-098). The reader does not check the entry's `request_index`
or `backend` either. The driver joins the entry to its item by
`request_index`, and ADR-029 PL-7's failures cover an entry for an item that
was not routed to the plugin.

#### Diagnostics

`PluginResultRefusal::diagnostic(item_index, received)` builds the
outcome-document diagnostic (`OutcomeDiagnostic::new`, ADR-029 CB-4). Its
detail names the result item index, the member or pointer, and the received
spelling or JSON kind. Every code comes from QSpec FR-272's closed table:

| Refusal | Code / cause |
| --- | --- |
| `QslOnlyCause`, `CoreOnlyBasis`, `UnknownSpelling` | `invalid_result`/`unknown-cause` at `/basis/type` or `/cause/type`: a spelling outside FR-331's closed set, or a core-only basis or cause, which QSpec FR-331 "Result entries" excludes from a plugin's entry (its core-only causes are this requirement's QSL-only causes) |
| `MissingMember` | `invalid_result`/`missing-member` |
| `MissingCounterexample` | at `/counterexample`: `invalid_result`/`missing-member` for an absent one, `invalid_result`/`wrong-value-kind` for one that is not an object; at `/counterexample/type`: `missing-member` for an absent `type`, `wrong-value-kind` for a `type` that is not a string, `invalid-value` for any other `type`, since a plugin's counterexample is assignments (QSpec FR-305, FR-331 "Result entries") |
| `UnknownMember` | `invalid_result`/`unknown-member` |
| `Malformed` | `invalid_result`/`wrong-value-kind` when the member's JSON kind is not the kind its domain takes (a non-object `basis`, a string `depth`, a non-string `certification`); otherwise `invalid_result`/`invalid-value` |
| `CertificationNotTrusted` | `invalid_result`/`certification-mismatch`, with `expected` `trusted` and `actual` the received value |
| `Unsupported` | `unknown_required_feature`/`unsupported-feature`: the feature is the received spelling, and the consumer is plugin settlement (QSpec FR-331 "Result entries") |

Every refusal but `Unsupported` carries its pointer, which the steps above
give, and its `invalid_result` diagnostic names that pointer as
`document_pointer` (QSpec FR-272, FR-331-AC-34). `Malformed` and
`MissingCounterexample` carry which of their causes applies.

`read_plugin_cause_kind` keeps its narrower contract (FR-289-AC-11,
FR-289-AC-13). `read_plugin_inconclusive_cause` applies it at step 3.
`TerminalRecord::from_plugin_proof` and
`TerminalRecord::from_plugin_inconclusive` are still the settlement steps.
They take the values these readers return.

### Replay context

A plugin counterexample settles only through its S6a replay (FR-289 item 4).
FR-098's `replay` decides, for a `function` request, the claim its occurrence
key selects. QSL replays a plugin's canonical assignments for two kinds of
item, each at the item's own occurrence:

- A **function-contract item**: an item whose occurrence key is the
  `declaration` occurrence (the function's node, role `declaration`,
  ordinal 0) of a checked function declaration whose result type is
  `Boolean`. That is ADR-013 O-09's function-contract subject, which
  FR-121's `call_site` locates. Its replay decides FR-098's predicate claim:
  a `false` result reproduces the violation.
- An **application item**: an item whose occurrence holds a
  `value-validity` record at a scalar operation application (FR-062) whose
  owner is a function declaration of the checked unit. Its claim is that
  the application is defined and its result lies in its result bound. Its
  replay calls the owner and decides FR-098's application claim at the
  item's occurrence, so the item is refuted only when the application is
  undefined or out of its bound under the assignments. Predicate replay of
  the owner would not test that claim: `p(x: Int[0, 9])` returning `x > 3`
  returns `false` at `x = 0` while the claim of its `>` application holds.
  The assignments join the owner's declared parameters by FR-098's rules.
  `replay` binds an owner parameter the record reads nowhere and the
  assignments leave unbound, when its type is `Boolean` or an integer type
  (FR-098 "unread parameters"). An assignment to a binder root of the
  record's extent, and one that leaves unbound an owner parameter that is an
  extent root, is a replay refusal.

QSL's request writer writes no item at a `declaration` occurrence: a `Value`
function declaration carries no claim of its own, and its claims are its
scalar operation applications (FR-057 claim-form table, FR-062 part 4).
So the plugin counterexamples that QSL's own routing produces replay as
application items.

Every other item is a **non-replayable item**: an `operation-contract` or
`temporal-satisfaction` record, which has no owner (FR-062), and a
`value-validity` record whose owner is a function the checker synthesizes
from an operation `body` or a `pre` clause, which `replay` cannot select by
name (FR-062). A non-replayable item's counterexample is not replayed. The
item settles `inconclusive` with cause `replay-refused` (FR-331's closed
cause, ADR-029 PL-7) and the catalog code `unknown_required_feature`. That
is the code FR-098 gives a construct its replay does not support. The record
keeps no replay result.

`PluginReplayContext`, in `qsl-replay` and re-exported at the crate root,
holds the package-wide members of the request. The driver supplies no member
of its own. `PluginReplayContext::new` takes:

- the `CheckedUnit` from `check`;
- the `EmittedUnit` that `package` emitted from it;
- the `AdmittedModels` from `select`;
- the run's `CallerLimits` (FR-255), whose `spine` limits the compile ran
  under (FR-278).

It copies what it needs and refuses nothing. A pair of units that do not
match is caught by the replay's own recompile, which requires the recompiled
`package_id` to equal the request's (FR-098).

`PluginReplayContext::request(occurrence, proof_bounds, plugin, assignments)`
builds the `ReplayRequestWire` for one counterexample. It takes:

- the routed item's `OccurrenceKey` and the substituted `ProofBound`s of its
  extent, which the driver wrote into the FR-331 request item (QSpec FR-331
  "Items");
- the plugin's `BackendId`;
- the assignments that `read_plugin_assignments` returned.

| `ReplayRequestWire` member | Value | Source |
| --- | --- | --- |
| `profile_selections` | empty: the checked package's lock `profile_selections` rows are clause-profile laws (a role and a definition), not FR-071 semantic profile selections, so the context carries none | ADR-013 O-25, FR-071 |
| `package_id` | the emitted package's `package_id`, in `quire.package.semantic/v2` | FR-071, FR-098 |
| `source_digests` | the unit's own source reference, the one source the recompile reads | FR-071, FR-098 |
| `dependencies` | one entry per resolved library selection of the checked package (the lock's direct `dependency_selections` and the libraries they import in turn, which the replay's recompile also needs), in ascending identity order: its `identity`, its `package_id`, and as `sources` the reference of the library source that `check` resolved for it (`CheckedUnit::libraries`) | FR-071, ADR-015 D-4 |
| `selected_function` | for a function-contract item, the qualified name of the function whose `declaration` occurrence the item names; for an application item, its record's owner | ADR-013 O-09, FR-062, FR-098, FR-121 |
| `occurrence_key` | the routed item's `OccurrenceKey`, its node as a `WireNodeId` | ADR-013 O-25, FR-071, FR-098 |
| `source` | `ReplaySource::Input(assignments)` | ADR-029 PL-7, ADR-013 O-25 |
| `obligation_identity` | the `process-provider` obligation identity below | ADR-013 O-09 |
| `backend` | the plugin's `BackendId` string | ADR-013 O-19, ADR-029 PL-5 |
| `state_environment` | empty: the FR-331 request carries no state environment, so the plugin run started from none | FR-071 |
| `accounting_limits` | the run's accounting budgets, `CallerLimits::accounting` | FR-071, FR-255 |
| `stage_limits` | every FR-255 stage setting of the `CallerLimits` (`SettingLimits::bounds`) except `replay.input_bytes` and `format.output_bytes`, which a request does not carry (FR-071). This covers the spine, library, admission, observation and exploration settings, so the replay compiles under the limits the package was compiled under and never under QSL's defaults. | FR-071, FR-263, FR-399 |
| `supplied_limits` | `None`. The call converts its arguments under the published supplied-value ceilings (QSpec FR-323 `limits.supplied`), because the compile takes no supplied ceilings and the FR-331 request carries none. | FR-071 |
| `object_environment` | `None`: canonical assignments carry no snapshot | FR-071, FR-098 |
| `declared_domains` | `DeclaredDomain::new(bound)` for each item proof bound, in the item's order | FR-071, ADR-014 B-4 |
| `byte_provision` | each source of `EmittedUnit::sources()` under its raw-byte digest, and each domain package document that `AdmittedModels` holds under its `sha256-jcs` digest | FR-071, FR-278 |

The request carries `s1.input_bytes` from the `CallerLimits`. FR-098 refuses
a request whose `s1.input_bytes` is above the `ReplayLimits` the driver passes
to `replay` (`LimitAboveReader`), and that refusal settles through
`TerminalRecord::from_plugin_counterexample`. So the driver passes the
`ReplayLimits` of the same `CallerLimits` (`CallerLimits::replay`).

The `process-provider` obligation identity is the SHA-256 of the RFC 8785
encoding of an object with these members. The encoding goes through ADR-013
§2's one encoder.

- `declared_domains`: the item's proof bounds, in O-09's parity `bounds` form
  (`{"tag": "bounds", "entries": [{key, bound}]}`, with entries ascending by
  the encoded `key`);
- `node`: the hex of the occurrence key's node;
- `obligation_kind`: `value-validity`, the QSpec FR-290 capability kind that
  QSL FR-057 gives a Boolean predicate's claim;
- `occurrence_key`: `{ordinal, role}`;
- `subject_kind`: the string `process-provider`.

A process item has no CG obligation (ADR-029 PV-4 ruling 3), so QSL computes
this identity itself. The `subject_kind` tag keeps it distinct from every CG
obligation form. The encoding rules are O-09's parity rules.

`request` checks in this order, and returns the first `PluginReplayRefusal`:

1. The occurrence key is neither the `declaration` occurrence of a checked
   function declaration nor the occurrence of a requirement record of the
   checked package: `UnknownOccurrence`, `missing_declaration`/`missing-name`.
   The refusal retains the key.
2. The item is a non-replayable item, as defined above: `NotReplayable`. The
   refusal retains the occurrence key.
3. A resolved library selection of the checked package has no library
   source that `check` resolved: `UnresolvedDependency`,
   `missing_import`/`missing-selection`. The refusal retains the entry's
   `identity`. This is the code FR-071-AC-5 gives a source reference with no
   provision. The step is a defensive check: the selections and
   `CheckedUnit::libraries` come from the same `CheckedUnit`, and `check`
   resolves every library it selects, so consistent inputs never reach it,
   and no acceptance criterion builds it.
4. Two proof bounds name the same `DomainKey`, which O-09 forbids
   ("no two entries having an equal `key`"): `DuplicateBoundKey`,
   `invalid_runtime_input`/`duplicate-member`. The refusal retains the key.
   Every other preimage member is a decimal string or a `u32`, so the
   encoder can refuse nothing else; if an invariant of the checked package
   fails here, `request` returns `Fault`, which carries an `InternalFault`
   (`runtime_invariant`/`established-invariant-broken`) and settles the item
   `failed`.

`PluginReplayRefusal::record(request_index, plugin)` settles the item and
names the plugin's `BackendId`:

- `NotReplayable` settles `inconclusive` with
  `InconclusiveCause::ReplayRefused(unknown_required_feature)` and keeps no
  replay result.
- Every other refusal settles `failed`. Its diagnostic goes in the outcome
  document, built by `PluginReplayRefusal::diagnostic(item_index)`, as for
  `PluginResultRefusal`.

`request` does not decode or verify the request it builds. `replay` reads it
through `ReplayRequest::decode` (FR-071). Every later FR-071 and FR-098
refusal settles through `TerminalRecord::from_plugin_counterexample`
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
- When a process provider reports a counterexample for a function-contract
  item or an application item, QSL shall replay it through layer-6 `replay`
  (FR-098) at the item's occurrence and settle the item `refuted` only if
  the replay reproduces it.
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
  `read_plugin_inconclusive_cause` and `read_plugin_assignments` shall each
  accept exactly the wire values that "Reading a plugin result member"
  states for that reader.
- `read_plugin_basis` and `read_plugin_inconclusive_cause` shall refuse a
  member they do not accept in the order of steps 1 to 8 in "Basis and
  cause", returning the `PluginResultRefusal` that the steps name.
- `read_plugin_basis` shall refuse a core-only basis by its spelling alone,
  whatever its payload.
- `read_plugin_inconclusive_cause` shall refuse a QSL-only cause by its
  spelling alone, whatever its payload.
- `read_plugin_certification` shall refuse a member it does not accept in the
  order "Certification" gives.
- `read_plugin_assignments` shall refuse an entry it does not accept in the
  order "Assignments" gives.
- Each `PluginResultRefusal`'s diagnostic shall carry the FR-272 code that
  the diagnostics table gives.
- `PluginReplayContext::request` shall build each `ReplayRequestWire` member
  from the source that the replay-context table gives, and shall take no
  other input.
- When the occurrence names neither a checked function declaration nor a
  requirement record, or names a non-replayable item, or a resolved library selection
  has no resolved source (a defensive check), or two proof bounds share a
  domain key,
  `request` shall refuse in that order and return no request.
- When `request` refuses `NotReplayable`, QSL shall settle the item
  `inconclusive` with cause `replay-refused` and code
  `unknown_required_feature`, naming the plugin's `BackendId`, with no
  replay result.
- When a reader or `request` returns any other refusal, QSL shall settle the
  item `failed`, naming the plugin's `BackendId`, and shall put the refusal's
  diagnostic in the outcome document.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-290-AC-1 | A process-provider FR-331 result of `proved` for a routed item settles a terminal record with value `proved`, the provider's `BackendId` and certification `trusted`; the FR-281-AC-1 record's certification is `certified`, naming the EN-5 checker; FR-281-AC-4's record from the test engine with no checker has certification `uncertified`, naming that engine; a Kani `proved` record built as C-09 settles it carries `certified`. | Test (TC-774) |
| FR-290-AC-2 | A process-provider counterexample that replay reproduces settles `refuted`; one whose assignment replay evaluates to `true` settles `inconclusive` with cause `replay_parity`; one naming a parameter the function lacks settles `inconclusive` with cause `replay_refused`. | Test (TC-774) |
| FR-290-AC-3 | A process-provider result that fails the FR-331 reader builds no terminal record from its body, and its items settle as FR-291 states. | Test (TC-774) |
| FR-290-AC-4 | The text rendering (FR-298) of AC-1's plugin record shows `proved` with the label `trusted` beside it, and its JSON outcome document holds the certification `trusted`; the `uncertified` record renders with the label `uncertified` beside `proved`; the AC-1 Kani record renders `proved` with the label `certified` beside it. | Test (TC-774) |
| FR-290-AC-5 | `read_plugin_basis` reads `{"type":"exhaustive"}` as `Exhaustive`, `{"type":"bounded-complete","depth":4}` as `BoundedComplete{depth:4}`, `{"type":"inductive","depth":0}` as `Inductive{depth:0}`, `{"type":"bounded-solver","horizon":{"denominator":"2","numerator":"5"},"jumps":3,"method":"delta-complete"}` as `BoundedSolver{DeltaComplete, 5/2, 3}`, the same basis with `method` `"flowpipe-unrounded"` as `BoundedSolver{FlowpipeUnrounded, 5/2, 3}`, and `{"type":"reduced","reductions":[{"parameters":{"proviso":"breadth-first-revisit"},"reduction":"partial-order","row":"invariant"},{"parameters":{"classes":[["a","b"]]},"reduction":"copy-swap","row":"hyper"}]}` as `Reduced` holding `PartialOrder{BreadthFirstRevisit}` then `CopySwap{[["a","b"]]}`. Each value it reads, passed to `TerminalRecord::from_plugin_proof`, settles `proved` with `trusted`, naming the plugin. | Test (TC-774) |
| FR-290-AC-6 | `read_plugin_basis` refuses each of these, with the refusal and pointer given: <br>- each of the six core-only spellings with its valid FR-331 payload, and `{"type":"closed-form","extra":1}`: `CoreOnlyBasis` at `/basis/type` (the spelling is read before the members); <br>- `{"type":"replay-parity"}` and `{"type":"bounded_complete","depth":4}`: `UnknownSpelling` at `/basis/type`; <br>- `"exhaustive"`: `Malformed` at `/basis`; <br>- `{"depth":4}`: `MissingMember` at `/basis/type`; <br>- `{"type":7}`: `Malformed` at `/basis/type`; <br>- `{"type":"inductive"}`: `MissingMember` at `/basis/depth`; <br>- `{"type":"exhaustive","depth":3}`: `UnknownMember` at `/basis/depth`; <br>- `{"type":"inductive","depth":"3"}`, `{"type":"inductive","depth":-1}` and `{"type":"inductive","depth":9007199254740992}`: `Malformed` at `/basis/depth`; <br>- a `bounded-solver` with `method` `"dreal"`: `Unsupported` at `/basis/method`; with `method` `""`: `Malformed` at `/basis/method`; <br>- a `bounded-solver` with horizon `{"denominator":"1","numerator":"-1"}` or `{"denominator":"4","numerator":"2"}`: `Malformed` at `/basis/horizon`; <br>- a `reduced` whose first reduction is `symmetry` with one valid FR-382 declaration: `Unsupported` at `/basis/reductions/0/reduction`; <br>- a `reduced` whose first reduction is a `copy-swap` with the class `["a","a"]`: `Malformed` at `/basis/reductions/0/parameters/classes/0`. | Test (TC-774) |
| FR-290-AC-7 | `read_plugin_certification` accepts `"trusted"`. It refuses an absent member with `MissingMember` at `/certification`, `7` and `"vouched"` each with `Malformed` at `/certification`, and `"certified"` and `"uncertified"` each with `CertificationNotTrusted` at `/certification`. For a `proved` entry with `certification` `"certified"` and basis `{"type":"bounded-proof","checks":3}`, the driver's reading order returns `CertificationNotTrusted`. | Test (TC-774) |
| FR-290-AC-8 | `read_plugin_inconclusive_cause` reads: <br>- each payload-free spelling of its table as its variant; <br>- `{"type":"bound-reached","depth":7}` as `BoundReached{7}`; <br>- `{"type":"reduction-not-preserving","form":"liveness","reduction":"constraint"}` as `ReductionNotPreserving{StateConstraint, Liveness}`; <br>- `{"type":"constraint-reached","boundary_states":2}`; <br>- `{"type":"reduction-horizon","max_depth":5}`; <br>- `{"type":"memory-bound-reached","limit":"max_messages","states":3,"value":2}` as `MemoryBoundReached{MaxMessages 2, 3}`; <br>- `{"type":"instance-bound-reached","limit":"max_live_instances","states":1,"value":4}`; <br>- a `mapping-undetermined` with `row` `"r"`, a 64-hex `state_key` and an empty `prefix` as `MappingUndetermined`. <br>Each value it reads, passed to `TerminalRecord::from_plugin_inconclusive`, settles `inconclusive` with it. | Test (TC-774) |
| FR-290-AC-9 | `read_plugin_inconclusive_cause` refuses each of these, with the refusal and pointer given: <br>- `replay-parity`, `replay-refused` and `kani_vacuous_proof`, each also with an extra member, and `stale-dependency` with its valid `requested` and `recompiled` members (QSpec FR-331-AC-34): `QslOnlyCause` at `/cause/type`; <br>- `{"type":"closed-form","analysis":"edf"}`, `kani-vacuous-proof`, `scalar-agrees` and `bound_reached`: `UnknownSpelling` at `/cause/type`; <br>- each of the eight causes of its `Unsupported` list, each with its valid FR-331 payload: `Unsupported`; <br>- a `reduction-not-preserving` with `form` `"fairness"`: `Unsupported` at `/cause/form`; <br>- `{"type":"constraint-reached","boundary_states":0}`: `Malformed` at `/cause/boundary_states`; <br>- `{"type":"memory-bound-reached","limit":"max_store","states":1,"value":1}`: `Malformed` at `/cause/limit`; <br>- a `mapping-undetermined` whose `state_key` is `"abc"`: `Malformed` at `/cause/state_key`; <br>- a `mapping-undetermined` whose `prefix` is `[{"transition":{"type":"terminal-stutter"}}]`: `MissingMember` at `/cause/prefix/0/post_state`; <br>- `{"type":"no-instance","depth":1}`: `UnknownMember` at `/cause/depth`. | Test (TC-774) |
| FR-290-AC-10 | `read_plugin_assignments` reads `{"backend":"acme","counterexample":{"type":"assignments","values":[{"parameter":"<64 hex>","value":{"type":"integer","value":"0"}}]},"request_index":0}` as one `CanonicalAssignment` of that `WireNodeId` and `ExactInteger(0)`, and an empty `values` as no assignment. It refuses each of these, with the refusal and pointer given: <br>- `[]`: `Malformed` at `""`; <br>- an entry naming `transcript` with no `counterexample`, and an entry with no `counterexample`: `MissingCounterexample` at `/counterexample`; <br>- an entry with a valid `counterexample` and a `transcript`: `UnknownMember` at `/transcript`; <br>- one whose `counterexample` is `7`: `MissingCounterexample` at `/counterexample`; <br>- one whose `counterexample` has no `type`, and one whose `counterexample` is a `TemporalCounterexample`: `MissingCounterexample` at `/counterexample/type`; <br>- an entry with an `extra` member: `UnknownMember` at `/extra`; <br>- a counterexample with no `values`: `MissingMember` at `/counterexample/values`; <br>- a counterexample whose `values` is `{}`: `Malformed` at `/counterexample/values`; <br>- an element with no `value`: `MissingMember` at `/counterexample/values/0/value`; <br>- a `parameter` of 63 hex digits or with an uppercase digit: `Malformed` at `/counterexample/values/0/parameter`; <br>- a value `{"type":"integer","value":"07"}`: `Malformed` at `/counterexample/values/0/value`; <br>- a second element repeating the first's parameter: `Malformed` at `/counterexample/values/1/parameter`. | Test (TC-774) |
| FR-290-AC-11 | The diagnostic of each refusal kind in AC-6 to AC-10 carries the code and cause the diagnostics table gives, `Malformed` and `MissingCounterexample` the cause of their case. Its detail names the result item index, the received spelling or JSON kind, and, for every `invalid_result` cause, `document_pointer`: the refusal's pointer, such as `/basis/type` for `CoreOnlyBasis`; `CertificationNotTrusted`'s also names `expected` `trusted` and `actual` the received value. A refused member's item settles `failed`, naming the plugin, with no diagnostic in the record. | Test (TC-774) |
| FR-290-AC-12 | Take a unit compiled through `parse`, `select`, `check` and `package` whose Boolean function `p(x: Int[0, 9])` returns `x > 3`, and whose `Integer` function `q(x: Int[0, 9])` returns `x + 1`. Build the context from its units and limits. <br>- For the function-contract item at `p`'s `declaration` occurrence with assignment `x = 0` (a hand-built item: QSL's request writer writes no item at a `declaration` occurrence), `request` gives a `ReplayRequestWire` whose members are those of the replay-context table: `source_digests` the unit's one source, `selected_function` `p`, `occurrence_key` `p`'s `declaration` occurrence, `backend` the plugin's identity, `declared_domains` the item's proof bounds, an empty state environment, no object environment, and a byte provision holding the unit's source. `replay` of it settles the input arm reproduced with category violation, and `TerminalRecord::from_plugin_counterexample` settles `refuted`, naming the plugin. <br>- The same item with `x = 5` settles `inconclusive` with cause `replay-parity`. <br>- For the application item of `q`'s `x + 1` with assignment `x = 9`, `request` gives a request whose `selected_function` is `q` and whose `occurrence_key` is the item's; `replay` completes `10`, inside the result bound `Integer`, and the item settles `inconclusive` with cause `replay-parity`, naming the plugin. | Test (TC-774) |
| FR-290-AC-13 | The `process-provider` preimage for node `11` repeated 32 times, occurrence `{"ordinal":0,"role":"declaration"}`, and one bound `integer_range` 0 to 9 at `DomainKey::Node` `22` repeated 32 times with an empty path, encodes to `{"declared_domains":{"entries":[{"bound":{"lower":"0","tag":"integer_range","upper":"9"},"key":{"node_id":"2222222222222222222222222222222222222222222222222222222222222222","path":[],"tag":"node"}}],"tag":"bounds"},"node":"1111111111111111111111111111111111111111111111111111111111111111","obligation_kind":"value-validity","occurrence_key":{"ordinal":0,"role":"declaration"},"subject_kind":"process-provider"}`, whose SHA-256 is `3df483b143782f27840dd0d42eca97ba51fdb60c01cb3ddbc48db020de6ac899`. With no bound, the digest is `95905de0c8797755b51edf13b26a4fcdb41908a1e55f43b60784e280fa8e3a1c`. AC-12's request for `p` carries the identity of its item's preimage. | Test (TC-774) |
| FR-290-AC-14 | The context built with `CallerLimits` whose `s1.tokens` is 200000, whose `library.definitions` is 9000, and whose accounting `work_units` is 5000, gives a request whose `stage_limits` holds `s1.tokens` 200000 and `library.definitions` 9000. Every other stage setting except `replay.input_bytes` and `format.output_bytes` is at its bound, and `accounting_limits` has `work_units` 5000. `request` refuses each of these, in this order: <br>- an occurrence key that is no function declaration and no record of the package: `UnknownOccurrence` (`missing_declaration`/`missing-name`); <br>- a state clause's `operation-contract` record: `NotReplayable`, which comes before `DuplicateBoundKey` for the same call when the record's call also carries two proof bounds at one `DomainKey`; <br>- for `p` with two proof bounds at one `DomainKey`: `DuplicateBoundKey` (`invalid_runtime_input`/`duplicate-member`). <br>Each refusal other than `NotReplayable` settles the item `failed`, naming the plugin. | Test (TC-774) |
| FR-290-AC-15 | Take FR-290-AC-12's unit with `function r using v(x: Int[0, 9]): Integer pure { 10 mod x }` added. <br>- For the application item of `r`'s `10 mod x` with assignment `x = 0`, `request` gives a request whose `selected_function` is `r`, whose `occurrence_key` is the item's `expression` occurrence and whose `obligation_identity` is the `process-provider` identity of that node and occurrence; `replay` finds the application undefined, settles the input arm reproduced with category violation, and `TerminalRecord::from_plugin_counterexample` settles `refuted`, naming the plugin. <br>- The same item with `x = 3` (`1`) settles `inconclusive` with cause `replay-parity`. <br>- For the application item of `p`'s `x > 3` with assignment `x = 0`, the item settles `inconclusive` with cause `replay-parity`, while AC-12's function-contract item of `p` with the same assignment settles `refuted`. <br>- The same item of `r` with `x = 12` settles `inconclusive` with cause `replay-refused` and the code `invalid_runtime_input` (`WrongValueKind`). <br>- Over a unit holding FR-395-AC-12's operation `body` (`body renamed using v on Config::ConfigVersion::add(n) { n + deref(self).versionNumber }`), `request` for the `value-validity` item of its `+`, whose owner is the function the checker synthesizes from the `body`, refuses `NotReplayable`, and the item settles `inconclusive` with `ReplayRefused(unknown_required_feature)`, naming the plugin. | Test (TC-774) |

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
- [FR-057](FR-057-admit-shared-capability-kinds.md): the claim-form table, under which a `Value` function declaration carries no claim of its own.
- [FR-121](FR-121-locate-a-function-call-site-through-the-replay-facade.md): the function-contract subject.
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
