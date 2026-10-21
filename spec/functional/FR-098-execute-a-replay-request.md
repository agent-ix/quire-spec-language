---
id: FR-098
title: "Execute a replay request through the layer-6 replay facade"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-010
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-142
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-071
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-072
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-027
    type: traces_to
  - target: ix://agent-ix/quire-specification/FR-323
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-106
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-364
    type: depends_on
---
# FR-098: Execute a replay request through the layer-6 replay facade

## Description

QSL SHALL provide the replay executor entry, `qsl_replay::replay`, as the
layer-6 `replay` facade's public API (ADR-013 O-26, C-13, TK-01; ADR-011
§2.1 E9, §6.1). It takes an FR-071 replay request. It recompiles the proved
package's one source unit through the spine (S1 to S4, the same
the FR-278 composition the CLI `compile` uses, FR-027) from the
request's digest-addressed byte provision, against the dependency input the
package reference's `dependencies` entries name (ADR-015 D-4). It requires
the recompiled `package_id` to equal the request's, selects the function
by its `QualifiedName` in the recompiled package's declarations, joins the
replay source's arguments to the function's parameters by parameter node
id, calls the function through S6a (`CheckedPackage::call`), and settles
an FR-072 replay result. The counterexample a request replays refuted its
property, so the proved verdict is `violation`.

The spine compile is public only because `command`, another crate, calls
it. It is not part of the facade CG may call: the FR-060 T12-A rule
refuses any CG reference to `qsl_replay::spine` -- a `use` of it, a path
through it, or a crate alias or glob import that reaches it.

A replay runs its call over an object environment. A request without
`object_environment` supplies none: the call runs over an environment with
unresolved references (`ObjectEnvironment::with_unresolved_references`), a
reference argument is admitted by its identity, and a read through one
completes no value (`unknown_required_feature`/`unsupported-feature`,
construct `dereference`), so it settles `inconclusive` with cause `NoValue`.
A request with `object_environment` supplies an admitted one: the executor
admits the snapshot document the member names by FR-106, a reference argument
is an object of that environment, and `deref(r).f` reads that object's field
and completes. The environment is the `ObjectEnvironment` FR-106's admission
builds, with the relationship extents and the population manifest FR-364
states. Replay holds no store of its own and no object the document does not
list.

## Inputs

- An FR-071 replay request, in its wire shape, with its optional
  `object_environment` (FR-071).

## Outputs

- An FR-072 replay result on the arm of the request's `ReplaySource`, or a
  structured refusal (`ReplayRefusal`) with no partial result.

## Behavior

- The executor SHALL obtain every source and domain-package input from the
  request's byte provision by digest, and SHALL read no path, environment
  variable or search location. Domain packages are the provision's
  `sha256-jcs` entries, handed to I1 as its package input.
- The executor SHALL refuse a request by the first of ADR-015 D-4's rules,
  each rule applied over all `dependencies` entries, in entry order, before
  the next rule, with QSpec FR-323's codes for the rules FR-323 states; the
  one-source rule and the dependency-input rule are QSL's own and run
  before the recompile.
- The executor SHALL refuse entries not in strictly ascending UTF-8 byte
  order of `identity`, a repeated identity included, as
  `ReplayRefusal::DependencySelections` (`invalid_package`/`invalid-value`
  at `/package/dependencies`), before it checks any source count or builds
  any dependency input.
- The package reference's `sources` SHALL name exactly one
  `quire.source.bytes/v1` source, the proved package's, and each of its
  `dependencies` entries (QSpec FR-323) SHALL name exactly one such source,
  that dependency's. The executor recompiles each source under its four
  FR-001 labels. A `sources` list or an entry naming a definition document,
  or other than one source, refuses: the recompile reads no definition
  document, and each package compiles from one unit.
- The executor SHALL read the optional `refinement_internals` value of the
  package reference, at `/package/refinement_internals`, and of each
  `dependencies` entry, at `/package/dependencies/<n>/refinement_internals`,
  and hand each unchanged to the recompile as FR-099's refinement internals
  input (the members FR-071 types). It SHALL NOT validate them itself: a
  defect surfaces from the recompile as `ReplayRefusal::Recompile`.
- The executor SHALL report the locus of such a refusal as the request
  pointer, `/package/refinement_internals` for the proved package's value
  or `/package/dependencies/<n>/refinement_internals` for the `n`th
  dependency entry's, followed by the pointer into the value that FR-147
  gives (QSpec FR-323-AC-17).
- A request that drops a non-empty value recompiles with the empty list, and
  the executor SHALL refuse it on the `package_id` comparison.
- The executor SHALL build the dependency input (FR-099) from the entries:
  each entry's `identity`, and its source's labels, identity
  as path and provided bytes. It SHALL carry a dependency-input refusal,
  such as two entries whose sources share one authority and identity, as
  `ReplayRefusal::DependencyInput` (`invalid_package`/`conflicting-definition`).
- The executor SHALL carry a refusal of the spine recompile (ADR-015 D-1) as
  `ReplayRefusal::Recompile`. A removed entry refuses as
  `missing_import`/`missing-selection`; when that import is in a
  library, the refusal is wrapped in `CompileRefusal::Dependency` with the
  library's path.
- After the recompile, and before comparing the proved package's
  `package_id`, the executor SHALL refuse an entry whose identity the recompiled package's
  `dependency_selections` does not hold as
  `ReplayRefusal::DependencySelections`, and then an entry whose
  `package_id` differs from the closure's selection of its identity as
  `ReplayRefusal::DependencyIdentityMismatch` (`stale_dependency`/`content-mismatch`), naming
  the identity, the entry's `package_id` (`requested`) and the recompiled
  one. An edited dependency source also changes the proved package's
  `package_id`, so these checks run first and name the stale dependency.
- The recompile SHALL run under the stage limits the request carries. The
  request's `stage_limits` maps each setting name to its bound, every entry
  passes through to its stage, and a setting with no entry runs at its
  published default
  ([FR-255](FR-255-name-the-setting-that-raises-a-reached-limit.md),
  [FR-263](FR-263-replay-at-any-depth-under-the-request-limits.md)). An
  `s1.input_bytes` bound above `replay.input_bytes` refuses before the
  recompile.
  Supplied admission/conversion ceilings SHALL instead use the paired
  QSpec FR-323 typed `limits.supplied` carrier; they are not recompile
  stage settings and SHALL NOT be passed through `stage_limits`.
- The recompiled `package_id` SHALL equal the request's. No
  `CheckedPackage` is built from wire bytes.
- The executor SHALL resolve the selection by name lookup in the recompiled
  package's declarations (OQ-5), and SHALL refuse a function whose declared
  result is not `Boolean` before any call (predicate replay; FR-357's
  value-parity entry replays a non-`Boolean` function). It SHALL convert each argument's
  `WireNodeId` to a `NodeKey` only by lookup in the recompiled package, and
  order the arguments by the function's declared parameter positions.
  `Input` assignments, and the `WitnessBinding` list the executor builds,
  may arrive in any order. A witness transcript's entries may not: FR-070
  requires them in ascending parameter node id and refuses any other order
  (`DecodeRefusal::EntryOrder`).
- Each argument SHALL arrive as a typed `WitnessValue` and become a value
  of its parameter's declared type. An `Input` assignment carries
  `WitnessValue::Boolean` for a `Boolean` parameter and
  `WitnessValue::Integer` for an integer type; a witness transcript entry
  reads as the parameter's binding type, `0` or `1` (`false`, `true`) for
  `Boolean` and a decimal integer for an integer type. An integer for a
  Boolean parameter, a Boolean for an integer one, or any value for a
  parameter of a kind no `WitnessValue` is, refuses before the call. A value outside a parameter's declared domain
  (`12` for `Int[0, 9]`) refuses at S6a admission. Both refuse with the
  same cause, `WrongValueKind` (`invalid_runtime_input`), naming the
  parameter's position.
- The executor SHALL take a `WitnessValue` tree for a parameter of any value
  type other than `Boolean`, an integer type and `Population<T>`: a leaf of
  enum, text, String, UUID, Timestamp, rational, decimal, float, quantity or
  reference type, or an
  `Option`, record, tuple, union or collection nested to any depth with
  leaves of any value family. It arrives as an `Input` assignment's value,
  or as a witness entry read as a `Canonical` binding in FR-070's witness
  value text. A `Population<T>` parameter is not a value and refuses
  `WrongValueKind` before the call, as before.
- The executor SHALL convert a `WitnessValue` tree to a kernel value of the
  parameter's declared type. It walks the value against the recompiled
  package's type environment on an explicit heap stack (ADR-030 D-1) and
  builds each node through the kernel's value constructors. It SHALL decide a reference's universe correspondence after its value
  shape and before its conformance, as FR-100 orders them. A
  reference whose `universe` differs from the universe of the declared
  `Reference<T>`'s object type (QSL FR-084) refuses
  `ReplayRefusal::ForeignReference` (`foreign_reference`/`foreign-universe`),
  naming the parameter's position and carrying the required and supplied
  universes as lowercase hex, before conformance is considered, so a reference
  that is both foreign-universe and nonconforming keeps this refusal. It SHALL
  refuse `WrongValueKind`, naming the parameter's position, before the call
  and with no charge, when:
  - a node's shape is not its declared type's kind (for example, a
    sequence for a `Set`, a record for an `Option`, a `float32` for a
    `Float64`, a decimal for a `Rational`, a String, UUID or Timestamp
    object for a parameter of any other declared type, or a `text` value for
    a `String`, `UUID` or `Timestamp` parameter, with no payload read as
    another kind);
  - an enum, record, tuple or union `name` is not the declared type's
    declaration node id, or a quantity's `unit` is not the declared unit;
  - an enum or union `member` is not a member of that declaration;
  - a record's fields are not exactly the declared fields in declaration
    order, or an absent or `null` slot is given for a field that does not
    admit it;
  - an arity differs from the declaration;
  - a reference's `object_type` names no object type of the model, or names a
    type that is neither the declared `Reference<T>`'s object type nor a type
    that conforms to it (QSpec FR-151; the reference admission of FR-100), so
    a proper subtype's reference is a `Reference<T>`; or a reference's
    `identity` is empty or not UTF-8, which the kernel's object identity
    constructor never accepts (FR-100);
  - a `set` or `ordered-set` holds two elements the kernel's equality
    relates (decimals compare by numeric value, so `1.0` and `1.00` are
    duplicates, though FR-070 gives them distinct value texts);
  - a decoded `set` or `ordered-set` whose elements bear a float. S3 admits
    no declared set type whose element type bears a float (QSpec
    FR-144-AC-6, FR-323-AC-3), so such a value matches no declared type and
    fails conversion as a kind mismatch.
- An enum member converts to the kernel member with its declared rank. A
  text converts to a kernel text of the declared text type. A String object
  converts to a kernel String of its exact scalar sequence, which no content
  refuses, and a UUID or Timestamp object converts through the kernel's
  canonical constructor, which refuses a noncanonical spelling
  `invalid_runtime_input`/`invalid-value` (FR-070, FR-056). A rational,
  decimal, float or quantity converts exactly, with no rounding. A
  reference converts to the kernel object reference of its three witness
  components, universe, `object_type` and identity, which a predicate replay
  holds as a value. A call that dereferences one completes a value only over
  an object environment the request supplies (below); with none it completes
  no value and settles `inconclusive` with cause `NoValue`.
- The executor SHALL admit a converted argument through S6a
  (`CheckedPackage::call`), the admission every runtime argument takes,
  under FR-106's input rule: a supplied input outside its declared domain is
  refused input, never evidence. A leaf outside its declared range (`12`
  in an `Int[0, 9]` field) or a collection outside its declared cardinality
  (four elements for `Sequence<Int[0, 9]>[0, 3]`) SHALL refuse
  `WrongValueKind` (`invalid_runtime_input`), naming the parameter's
  position, before the call. So do a text outside its declared length or
  profile, a decimal outside its declared range or scales, a rational
  outside its declared domain, and a float or quantity the declared type
  does not admit. Union arguments follow FR-321's admission.
- The conversion SHALL stop at the request's limits, never at a depth. Before
  the call, it SHALL count the occurrences of each converted argument
  (`quire.value.accounting/v1` `occ`) against the request's accounting
  limit `value_occurrences`, and the nodes it converts against
  `work_units`. These are `quire.value.accounting/v1` accounting limits
  (ADR-014 B-2), not stage limits. FR-255 names each by its counter name,
  which is its setting, and a `stage_limits` entry of that name replaces
  the request's accounting-limits value for that counter (FR-255
  Behavior 10). These checks, the call and every identity that binds the
  request's limits use the resulting effective accounting limits. When
  a count exceeds its limit, the replay SHALL settle, with no call, as
  FR-277 settles a pre-call accounting limit: a phase-marked
  `Incomplete`, naming conversion, the parameter, counter, configured value,
  successful count and denied next amount, without claiming an evaluated
  call. FR-098 settles this no-call outcome as
  `inconclusive` with cause `NoValue`. By deliberate choice these checks
  charge nothing to the call's meter. The call then consumes the same
  charges it did in the proving run, so replay parity compares like with
  like. The
  request's or envelope's bytes are bounded by `replay.input_bytes`, as
  [FR-263](FR-263-replay-at-any-depth-under-the-request-limits.md) states.
  The request's measured size counts each `Input` value tree at the byte
  length of its escaped FR-070 value text.
- Conversion SHALL retain the preceding per-argument semantic node and
  occurrence rules unchanged (QSL-681 settlement option 1). It SHALL bound
  helper work separately by public `supplied.conversion_work_units`, with one
  cumulative caller-owned budget for the entire argument list. QSL helper
  events SHALL follow the deterministic event table below. Conversion SHALL
  include a counted scheduling step only for QSL witness bookkeeping, never
  twice. Each required QSV child-scheduling event SHALL remain
  a separate unit under FR-109; bookkeeping SHALL neither absorb nor
  duplicate that delegated event.
  Conversion SHALL admit each helper step
  before its work, process iteratively and reserve storage fallibly
  before mutation, under FR-277's zero-ceiling/overflow/denied-step rules.
  Helper units SHALL NOT become converted-node units or evaluation charges.
- A converted node SHALL contribute exactly one unit to its argument's
  node counter C. Declaration descriptors, type-expression nodes, cache
  lookups, comparison steps and scratch entries SHALL NOT be converted
  nodes. Semantic occurrence counts SHALL use the existing `occ` rule,
  independently of whether membership trusts an admitted nested value.
  Conversion SHALL check counts before the operation that would exceed them,
  preserving the successful per-argument prefix and denied next amount.
- Membership and iterative type comparison SHALL use QSV's reviewed FR-109
  value-visit/type-comparison/child-schedule/descriptor-query event sequence
  and the initiating phase's cumulative budget: conversion when invoked
  from conversion, admission when invoked by direct/final admission.
  One actual event SHALL NOT debit both phases. The original caller
  Cancel SHALL reach conversion, nested helpers, membership and the eventual
  call. A denial SHALL preserve its owning phase and FR-277 typed payload,
  with no later argument converted or evaluated. Union-specific event
  integration SHALL await the reviewed QSV union declaration/member/type
  comparison baseline; no candidate event total defines this contract.

Each delegated type-comparison unit SHALL be exactly ONE compared PAIR OF
type-chain LINKS, including the initial expected/actual pair, with no extra
expected-type-entry charge. Neither each individual link nor an entire
chain replaces that unit. Each supplied-family path requires FR-277 AC-6's
reviewed helper capability and typed integration before implementation
qualification. Decimal positive replay remains mandatory once that owning
capability is enabled; missing IR-718/IR-719 proof is a qualification
blocker, not a newly invented runtime refusal or unsupported result.

### Conversion helper event units

Conversion SHALL process arguments in declared parameter order, nodes in
depth-first declared field/payload/element order, and materialization in
postorder. At each scheduled step it SHALL poll original Cancel, check the
owning budget, reserve required storage fallibly, and only then mutate
state. A denied or cancelled step SHALL perform none of its work. Successful
event spend SHALL be recorded only for work actually performed; a failed
reservation retains its original measured storage/capacity cause and the
successful prefix, never a fictitious limit.

| Event | Exactly one unit before | Boundary/order |
| --- | --- | --- |
| Witness preflight | Reading one witness node's discriminant and validating its local shape/arity | Before resolving its declared type or visiting children; first invalid node stops all later work |
| Descriptor inspection | One logical descriptor query for a declaration header, member or field/position under QSV FR-109 | Header before selected member/positions; declaration order; preserve logical count on cache hits and count a delegated query only once |
| Runtime type resolution | The actual logical compared-pair-of-type-chain-links or descriptor-query steps prescribed by QSV FR-109 for that resolution, including the initial expected/actual pair | No extra entry or facade-invocation debit for the same delegated step; independently bounded type/link/registry inputs constrain unhooked internal work under FR-277 |
| Witness materialization | Constructing one converted kernel node after its children | Postorder; no semantic evaluation charge; the same completed node counts once in the separate per-argument C counter |

Retrieving a descriptor is a descriptor event, not a second runtime-type
resolution event for that same read. Resolving a type expression reached
from it is a separate actual step. Membership checks and structural type
comparison SHALL retain the initiating phase's one budget owner; conversion
SHALL not recount the same operation as resolution. Nonconstant subordinate walks (including
leaf text/numeric validation, equality/deduplication and reference walks)
SHALL use the logical helper events and independently bounded helper inputs
of FR-277, with one budget owner. They SHALL NOT claim that one outer
event bounds an unbounded internal walk. Their input/event contract SHALL be agreed with the shared
helper owner before this contract is frozen; QSL prescribes no missing
union descriptor signature. Semantic conversion-node counting SHALL remain
one per converted node, independently of all such helper events.

"With no charge" in this requirement's pre-call refusal rules means no
semantic evaluation charge. Actual successful helper work before invalid
input is discovered SHALL remain reported in its owner budget; it SHALL
NOT produce a semantic ChargePoint or override the original refusal cause.

### The object environment

A function that takes a `Reference` reads the object it names through an
object environment. A counterexample over such a function supplies, besides
its arguments, the environment document: a `quire.state.snapshot/v1` document
(FR-106) listing the objects the argument references name and every object
reachable from them by reference fields, each with every declared field, in
populations marked `complete`. A reference argument arrives in FR-070's one
reference spelling. An object record never travels in a witness or an `Input`
assignment, so a record or any other value for a `Reference` parameter is a
node whose shape is not its declared type's kind and refuses `WrongValueKind`
as above. The executor SHALL decide the following in this order, each step
before the next, with no charge to the call's meter:

1. **Unused environment.** If the request carries `object_environment` and no
   parameter of the selected function has a type that contains a `Reference`,
   directly or nested in an `Option`, record, tuple, union or collection, then
   the executor SHALL refuse `ReplayRefusal::UnusedObjectEnvironment`
   (`invalid_runtime_input`/`unknown-member`, naming `object_environment`).
   It decides this after the argument join and before any conversion and
   before it reads the document. Such a function can dereference nothing.
2. **Conversion.** Every argument converts as above, a conversion refusal or a
   pre-call accounting settlement before step 3.
3. **Admission.** The executor SHALL admit the named document as FR-109's
   `Function` selection admits its current snapshot, reading it only from the
   byte provision by its `sha256-jcs` digest, under FR-106's default
   `ObservationLimits` (with `stage_limits` entries for their settings and
   post-state witnessing off), by these FR-106 checks in this order, stopping
   at the first failing condition: check 1; check 3 for its role condition
   alone, the document's `observation` being `current` (else
   `wrong_snapshot`/`wrong-observation`); check 4, with the document's `model`
   equal to a model selection of the recompiled package; and checks 6, 7 and
   8. The document's `anchor` is read for form and never compared, because a
   function has no anchor. Checks 2, 5, 9, 10 and 11 do not apply: there is no
   clause, invocation, `self` object, parameter map or result. The required
   populations of check 7 are the populations that hold an object a converted
   argument's reference names, found by its reference key, and, repeatedly,
   FR-106's closure over their reference fields and relationship ends. The
   admission also issues FR-364's population manifest and bundle, whose
   selection is the function's `QualifiedName` with the document's digest and
   whose observation role is `current`. A failing condition SHALL refuse
   `ReplayRefusal::ObjectEnvironment` holding FR-106's record, the code,
   cause and path it names, `Refused` or `Incomplete`, and the executor SHALL
   call nothing. A missing document is `unavailable_observation`/
   `missing-required-artifact`, a repeated object key `invalid_runtime_input`/
   `conflicting-identity`, an ill-typed field `invalid_runtime_input`/
   `wrong-value-kind`, a reference to an object a complete population does not
   list `dangling_reference`/`absent-target-in-complete-population`, and a
   required population not marked `complete` `incomplete_population`/
   `incomplete-scope`. Replay defines no environment grammar, code or cause of
   its own.
4. **Resolution.** For each reference in each converted argument, in parameter
   order and, within an argument, in the conversion's walk order (a reference
   nested in an `Option`, record, tuple, union or collection included), the
   executor SHALL decide FR-364's `resolve` over the bundle and SHALL stop at
   the first reference that is not `present`. A reference is `present` when
   the environment holds the object whose whole reference key (universe,
   type and identity) equals it, never one whose identity alone matches.
   `dangling` SHALL refuse `ReplayRefusal::ObjectEnvironment` with
   `dangling_reference`/`absent-target-in-complete-population`, and
   `undecidable` SHALL refuse it with `incomplete_population`/
   `incomplete-scope` as `Incomplete`; each names the parameter's position and
   the reference. These are FR-364's names, and resolution decides no charge.
5. **Call.** The executor SHALL call the function through S6a over the
   admitted environment, not over unresolved references. A `deref(r).f` of a
   held reference completes the field's stored value and the call settles as
   FR-072 states by the Boolean it completes. Every reference reachable from a
   held object is held, by FR-106 checks 7 and 8, so the call decides no
   `dangling` or `undecidable` of its own; a read of an object the environment
   does not hold is an internal fault and never a `NoValue`.

A request with no `object_environment` skips steps 1, 3 and 4 and runs as
the first paragraph of the Description states.

### Refusal and settlement

- Each ADR-013 O-26 refusal SHALL be a typed `ReplayRefusal` variant with no
  partial result: request decode refusals (FR-071), a limit above the
  reader limit, a source reference that is not one source unit, a
  `dependencies` entry whose `package_id` differs from the recompiled
  closure's selection of its identity (`DependencyIdentityMismatch`), a
  `dependencies` list that is not strictly ascending or holds an entry the
  closure does not (`DependencySelections`), a dependency-input refusal
  (`DependencyInput`), a recompile
  refusal or stage limit (`stage_limit_exceeded`), a stale `package_id` (`stale_dependency`/`content-mismatch`, carrying the request's and the recompiled `package_id`), a
  selection naming no function node, an argument naming no parameter, a
  parameter bound twice or not at all, an `object_environment` the selected
  function cannot use (`UnusedObjectEnvironment`), an environment document or
  a reference argument that FR-106's admission or FR-364's `resolve` refuses
  (`ObjectEnvironment`), a witness that does not decode
  (naming the request's `obligation_identity`), an S6a
  admission refusal (wrong type, or a value outside the declared
  domain), and a selected function whose declared result is not
  `Boolean`. Each has a catalog code (`ReplayRefusal::code`).
- A replay whose verdict differs from `violation` SHALL settle
  `inconclusive` with both verdicts as its typed cause (`Verdicts`), and one
  whose S6a outcome completes no value (`refused`, `incomplete`,
  `undefined` or a family result) SHALL settle `inconclusive` with cause
  `NoValue`, never repaired (FR-072).

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-098-AC-1 | A request whose byte provision carries its one source and, under their `sha256-jcs` digests, the domain packages the source selects recompiles from those bytes alone, keeps its `package_id`, and replays. A package reference naming a definition document or two sources refuses before any recompile. | Test (TC-444) |
| FR-098-AC-2 | The selection resolves by `QualifiedName` in the recompiled package; `Input` assignments and `Witness` entries join the function's parameters by parameter node id, in declared parameter order (`Input` assignments in any order; transcript entries in FR-070's ascending node-id order), and a Boolean parameter takes a witness entry `0` or `1` and an `Input` assignment's `WitnessValue::Boolean` (an `Input` `WitnessValue::Integer` for it refuses `WrongValueKind`); the call runs through S6a, and an agreeing replay settles `reproduced-without-witness` (`Input`) or `reproduced-with-evaluated-witness` (`Witness`) with no QSpec FR-351 record, since a function call's result has no decisive occurrence (ADR-031 SW-7), carrying the call's charges. | Test (TC-444) |
| FR-098-AC-3 | A meaning-affecting source edit refuses `stale_dependency`/`content-mismatch` by `package_id`, naming both identities. A presentation-only edit, which keeps the `package_id`, refuses by source digest. | Test (TC-444) |
| FR-098-AC-4 | Each refusal in Behavior -- a missing input, a byte/digest mismatch, a stale `package_id`, a selection naming no function node, an arity mismatch, a type mismatch and a value outside the declared domain (each `WrongValueKind`), a limit above the reader limit, a recompile stage limit at S1 or S3, and a selection whose declared result is not `Boolean` (refused before any call, even with no accounting budget) -- refuses with its typed variant and no partial result. | Test (TC-444) |
| FR-098-AC-5 | A replay that disagrees with the refuted property settles `inconclusive` with cause `Verdicts`, and one that completes no value with cause `NoValue`, each holding both verdicts; neither is repaired. | Test (TC-444) |
| FR-098-AC-6 | A request for a proved package importing `test/units`, whose `dependencies` entry names `test/units`, its `package_id` and its one source, replays from the byte provision alone. With that entry's source bytes replaced by an edit that changes `test/units`'s `package_id` (digests updated to match), the replay refuses `ReplayRefusal::DependencyIdentityMismatch` (`stale_dependency`), naming `test/units`, the entry's `package_id` and the recompiled one, with no verdict; with only the entry's `package_id` changed, it refuses `ReplayRefusal::DependencyIdentityMismatch` naming the same identity. | Test (TC-444) |
| FR-098-AC-7 | A request whose `dependencies` entries are swapped, or repeat one identity, refuses `ReplayRefusal::DependencySelections` (`invalid_package`/`invalid-value` at `/package/dependencies`) before any recompile; one carrying an extra entry no import reaches refuses `DependencySelections` after the recompile; two entries whose sources share one authority and identity refuse `ReplayRefusal::DependencyInput` (`invalid_package`/`conflicting-definition`); one lacking an entry refuses `ReplayRefusal::Recompile` carrying `missing_import`/`missing-selection` at the import; an entry naming two sources, or a definition document, refuses as a source reference that is not one source unit. None yields a verdict. | Test (TC-444) |
| FR-098-AC-8 | Over `record Inner { a: Int[0, 9]; b?: Boolean; }`, `union Shape { Circle(Int[0, 9]), Empty }` and `record Outer { i: Inner; o: Option<Int[0, 9]>; s: Sequence<Int[0, 9]>[0, 3]; u: Shape; }`, a predicate `p(x: Outer): Boolean` replays its counterexample from an `Input` assignment and from a `Witness` entry, and each settles as AC-2 states, with the replayed call evaluating the nested values. Each of these refuses `WrongValueKind` naming position 0, before the call and with no charge: a sequence given for `o`; an `Inner` whose `name` is `Outer`'s node id; a record missing `a`, and one with an undeclared field; an absent `a`; union member `Square`; `Circle` with no component; a leaf `12` for `a`; and four elements for `s`. Over `g(v): Boolean`, whose `v` is a `Set` of a `Decimal` type with scales 0 to 2, a set holding `1.0` and `1.00` (distinct value texts, numerically equal) refuses `WrongValueKind` the same way. | Test (TC-906) |
| FR-098-AC-9 | For the AC-8 counterexample and TC-906's unchanged full Tree, with other bounds permitting progress, occurrence-count-minus-one and converted-node-count C-1 make no call and settle `inconclusive`/`NoValue` with conversion-phase `Incomplete` preserving the configured `value_occurrences` or `work_units` and denied count. Counts at the bound admit the argument; successful replay also requires sufficient evaluation allowance. C remains converted nodes per argument, not helper visits, and contributes zero to the evaluation Meter. Direct/replay evaluation charge sequences are equal. | Test (TC-906) |
| FR-098-AC-10 | One predicate per leaf family, each taking one parameter of that family (an enum `Color`, `Text[0, 8]`, `Rational`, a `Decimal` with scales `[0, 2]`, `Float64`, a quantity in a declared unit `m`, and a `Reference<T>`), replays its counterexample from a `Witness` entry in FR-070's witness value text and settles as AC-2 states; the reference predicate tests the reference for equality with itself and does not dereference it. Positive replay requires FR-277 AC-6's reviewed capability and typed integration for the invoked helpers: Decimal remains a mandatory positive family once IR-718's reviewed capability is enabled through IR-719 and its typed mapping is approved. Missing proof blocks implementation qualification, not a runtime unsupported/invalid/limit/storage outcome. Each of these refuses `WrongValueKind` naming position 0, before the call: an enum value naming another enum's declaration; a nine-scalar text; a decimal with scale 3; a `float32` for the `Float64`; a quantity in another unit; and a reference whose `object_type` is another type's. A predicate that dereferences its reference settles `inconclusive` with cause `NoValue` when the request carries no `object_environment`. | Test (TC-906) |
| FR-098-AC-11 | A proved package whose one source declares `function wide using v(x: Int[0, 18446744073709551615]): Boolean pure { x <= 18446744073709551614 }` recompiles from its byte provision to the request's `package_id`, and an `Input` assignment of `x` = 18446744073709551615, carried on the replay wire as the decimal string `"18446744073709551615"`, settles `reproduced-without-witness` with the call's result `false`. The same holds for `x: Int[0, 9223372036854775808]` with body `x <= 9223372036854775807` and `x` = `"9223372036854775808"`, for `x: Int[0, 18446744073709551616]` with body `x <= 18446744073709551615` and `x` = `"18446744073709551616"`, and for `x: Int[-170141183460469231731687303715884105728, 170141183460469231731687303715884105727]` with body `x > -170141183460469231731687303715884105728` and `x` = `"-170141183460469231731687303715884105728"`. As the QSL-source stand-in for QSL-642's CG wide-range model (the u64 range CG admits under IR-624 AC35), a unit declaring `record Meter { reading: Int[0, 18446744073709551615]; }` and `function meter_over using v(m: Meter): Boolean pure { m.reading <= 18446744073709551614 }` recompiles from its byte provision to the request's `package_id`. | Test (TC-913) |
| FR-098-AC-12 | TC-906 counts the specified conversion helper events as H: H-1 denies its actual next event unspent, H permits conversion when other bounds fit. Across multiple arguments H is cumulative and independent of each argument's node C. Original Cancel, invalid input and storage/capacity failures retain their FR-277 typed phase causes and zero evaluation consumption. | Test (TC-906) |
| FR-098-AC-13 | TC-1028 E1 to E5 replay through the real spine. A unit declaring `function eqs using v(a: String, b: String): Boolean pure { a = b }`, its `!=` counterpart and `function equ using v(a: UUID, b: UUID): Boolean pure { a = b }` is composed and replayed from `Input` assignments that carry String values (the String wire object, `{"kind":"string","text":...}`), and each replay completes with the Boolean TC-1028 states as its evaluated value (the proved verdict is `violation`, so a `true` result settles `inconclusive` with cause `Verdicts` and a `false` result settles `reproduced-without-witness`): `""` and `""` give `true`; `"abc"` and `"abc"` give `true`; U+00E9 and U+0065 U+0301 give `false`; `"A"` and `"a"` give `false`; the text `00112233-4455-6677-8899-aabbccddeeff` and itself give `true`; the `!=` function gives the negation of each. Before the call and with no charge, each of these refuses `WrongValueKind` (`invalid_runtime_input`) naming the parameter's position, from an `Input` assignment and from a `Witness` entry: the String object whose text is `00112233-4455-6677-8899-aabbccddeeff` at position 0 of `equ`; a UUID object at position 1 of `eqs`; a Timestamp object at position 0 of `eqs`; `{"type":"text","value":"abc"}` at position 0 of `eqs`; and a String object at position 0 of the Text-parameter predicate of FR-098-AC-10. | Test (TC-1028) |
| FR-098-AC-14 | Over the fixture of TC-1042 (`Node` with `items: Sequence<Int[0, 9]>[0, 3]`, `Hub` specializing `Node`, a complete population `G`) and the environment document S1, `function fa using v(n: Reference<M::Node>): Boolean pure { forall(x in deref(n).items: x < 5) }` replays a reference to `a` (items 3, 7) to `false` from an `Input` assignment (`reproduced-without-witness`) and from a `Witness` entry (`reproduced-with-evaluated-witness`); a reference to `b` (items 1, 2) completes `true` and settles `inconclusive` with cause `Verdicts`; a reference to the `Hub` object `h` (items 9), whose `object_type` is the proper subtype, completes `false`; a sequence of references to `b` then `a` completes `false` through two reference reads (`forall` stops at the first false); and a different `anchor` in S1 changes none of these. | Test (TC-1042) |
| FR-098-AC-15 | With no `object_environment`, the call of `fa` over a reference to `a` settles `inconclusive` with cause `NoValue` (`unknown_required_feature`/`unsupported-feature`, construct `dereference`), while a predicate that only compares the reference with itself completes `true` and settles `Verdicts` with and without the environment. A `record` value for the `Reference` parameter refuses `WrongValueKind` naming position 0. | Test (TC-1042) |
| FR-098-AC-16 | A reference of `Lone`'s universe for the `Node` parameter refuses `ForeignReference` (`foreign_reference`/`foreign-universe`) naming position 0, whether or not its `object_type` conforms; a `Node` reference for a `Reference<Hub>` parameter (same universe, no conformance), a reference whose `object_type` names no object type, one with the empty `identity`, and one with the identity byte `ff` each refuse `WrongValueKind` naming position 0, before the environment document is read. | Test (TC-1042) |
| FR-098-AC-17 | A request carrying `object_environment` for a function with no `Reference` parameter refuses `UnusedObjectEnvironment` (`invalid_runtime_input`/`unknown-member`, naming `object_environment`), whether or not the document is in the byte provision, and before a wrong-kind argument of that function is converted. | Test (TC-1042) |
| FR-098-AC-18 | The environment document refuses with `ReplayRefusal::ObjectEnvironment` holding FR-106's record, one case per condition: a digest with no byte-provision entry (`unavailable_observation`/`missing-required-artifact`); a `pre` document (`wrong_snapshot`/`wrong-observation`); a `model` digest the package does not select (`invalid_model_binding`/`wrong-model-selection`); a repeated object key (`conflicting-identity`), reported before the dangling reference the same request's argument would raise; a non-sequence and an out-of-range element for `items` (`wrong-value-kind`, `invalid-value`); a reference field to an unlisted object of a complete population (`dangling_reference`/`absent-target-in-complete-population` at check 8); and an argument's population not marked `complete` (`incomplete_population`/`incomplete-scope` naming it at check 7). The call is never made. | Test (TC-1042) |
| FR-098-AC-19 | Against a complete population, a reference argument whose object the environment does not list refuses `dangling_reference`/`absent-target-in-complete-population` naming position 0 and the reference, and so does one whose `object_type` is another type's, within the same universe, from the held object of the same identity (a different universe refuses earlier, at conversion, as `foreign-universe`); against a population not marked `complete`, such a reference refuses `incomplete_population`/`incomplete-scope`. In a sequence of references the first one that is not present is named, and a present reference before it is not. | Test (TC-1042) |
| FR-098-AC-20 | A request for a proved package whose refinement `CasTwice` has the internal list `[{"refinement": "CasTwice", "internal": ["Spec::Twice::finish"]}]` carries it at `/package/refinement_internals` and recompiles to the proved `package_id`; the same request with the member removed refuses with `PackageIdMismatch` naming both identities; a request carrying a spelling that resolves to no class (`Spec::Twice::zz` at index 0) refuses `Recompile` with `missing_declaration`/`missing-name` located at `/package/refinement_internals/0/internal/0`, and the value `{}` refuses `invalid_model_binding`/`malformed-declaration` located at `/package/refinement_internals`; a defective value on `dependencies` entry 1 is located at `/package/dependencies/1/refinement_internals/<pointer>`; and a value on a `dependencies` entry applies to that dependency's refinements only. | Test (TC-444) |


The union cases of AC-8 and AC-9 depend on QSL-503's S6a union
arguments. A quantity-typed parameter in source (AC-10) depends on
QSpec FR-142's `Quantity<u>`, S1's `Quantity<qualified-name>` production and FR-091's unit-namespace type lookup; the quantity conversion is tested directly.

## Dependencies

- [FR-070](FR-070-implement-typed-counterexample-witness-envelope.md): the
  witness transcript, its witness value text and `decode`.
- [FR-106](FR-106-admit-snapshots-and-invocations.md): inputs out of their
  declared domain are refused input, and the admission of the object
  environment document;
  [FR-364](FR-364-carry-relationship-instances-in-an-observation-and-read-them-for-navigation.md):
  the population manifest, the bundle and `resolve`;
  [FR-109](FR-109-run-a-state-clause-through-the-spine.md): the `Function`
  selection's admission of one current snapshot;
  [FR-321](FR-321-admit-supplied-union-values.md): union argument admission;
  [FR-277](FR-277-bound-every-lifecycle-operation-by-caller-limits.md):
  `LimitExceeded`.
- [FR-071](FR-071-implement-typed-replay-request.md): the request and its
  decode-time refusals.
- [FR-072](FR-072-implement-typed-replay-result.md): the result and its
  settlement.
- [FR-027](FR-027-export-compiled-native-package.md): the spine compile.
- [FR-099](FR-099-compile-against-supplied-libraries.md): the dependency
  input and the S4 source resolution; ADR-015 D-4.
- ADR-013 O-25, O-26, C-11, C-13, OQ-5; ADR-011 §2.1 E9, §4, §6.1.
- QSpec FR-323 (`byte_provision`, `replay`).
