---
id: FR-121
title: "Locate a function's, an operation's or a state clause's call site through the replay facade"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-010
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-027
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-088
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-098
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-105
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-115
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-116
    type: depends_on
---
# FR-121: Locate a function's, an operation's or a state clause's call site through the replay facade

## Description

QSL SHALL provide `qsl_replay::call_site`, a second public entry of the
layer-6 replay facade (ADR-011 §6.1, ADR-013 TK-01) beside `replay`
(FR-098), for a consumer that must build an FR-071 replay request or an
FR-116 frame or state-clause counterexample itself rather than execute
one: the compiled package's own `package_id` and, for one selection, a
named function's node, its `declaration` occurrence key and its declared
parameters, each paired with its own node id; a
named operation's `operation_anchor` and `frame` nodes, the frame's
occurrence key and the node and occurrence key of every state clause
naming the operation; or a named state clause's node and occurrence key.

`call_site` compiles one unit through S1 to S4 -- the same
the FR-278 composition FR-027's `compile` command uses -- against the
supplied domain package documents and dependency input, under the default
spine stage limits. It resolves a function by the same one-segment name
lookup in the compiled package's declarations FR-098's selection uses
(OQ-5), an operation by the same typed `OperationName` resolution
FR-115's `Frame` selection uses, and a state clause by the declared name
FR-106's `ClauseSelection` names it by. It derives each node id the same way
`replay` and `replay_frame` do, so a request or counterexample keyed by
`call_site`'s answer is keyed the way those entries actually accept. It
builds no `replay` call and no `spine::Call`; it names a call site, it does
not make one.

CG needs this because it reaches QSL only through `qsl_replay` (ADR-011
FB-05) and may not name `qsl_replay::spine` (ADR-011 §3 FB-05, T-12 rule
(a): `spine` is public for `command` and the orchestrating driver), yet must know these identities
to build the harnesses and requests `replay` and `replay_frame` later
execute. For the same reason `qsl_replay` re-exports, at its root, the
types a caller constructs a `call_site` input from or matches a refusal
on: `DependencyInput`, `SuppliedLibrary`, `DependencyInputRefusal`,
`SourceHolder` and `OperationName`, and the kernel `Origin` and `Role`
that `OccurrenceKey::new(WireNodeId, Origin)` takes. It also
re-exports the types a caller builds a `DeclaredDomain` from: `ProofBound`,
`DomainKey`, `FiniteBound`, `FiniteBoundKind`, `EmptyFiniteBound`, and the
kernel `Integer`, `IntegerInterval` and `EmptyInterval`. A caller reads a
library identity, catalog code or host cause through the methods of the
value that carries it (`LibraryName::as_str`, `DependencyInputRefusal::code`
and `host_cause`).

Pairing each parameter with its declared name, and each clause's node with
its declared name, rather than returning node ids alone, is required by
ADR-013 O-25: a consumer joins a witness row or an argument to its
declaration by declared identity, never by position.

## Inputs

- `source`: the unit's FR-001 source identity.
- `path`, `bytes`: the unit's authored path and source bytes.
- `packages`: the domain package documents, each keyed by its own
  `sha256-jcs` digest as I1's package input, exactly as `replay` keys the
  documents of its byte provision.
- `dependencies`: the `DependencyInput` of supplied libraries (ADR-015
  D-1), built by `DependencyInput::new`.
- `selection`: a `QualifiedName` naming a function, an `OperationName`
  naming an operation `M::T::op` as model alias, object type and operation
  identifiers, or a `ClauseName` naming a state clause by its declared
  `Identifier`. The selection types are exactly the three implementors of
  the sealed trait `CallSiteSelection`.

## Outputs

- A `CallSite` on success: `package_id` (`DigestRecord`), `package`
  (`Vec<u8>`, the compiled package's `quire.checked-package/v2` bytes
  exactly as the S4 emitter wrote them when it minted `package_id`), and
  `site`, whose type the selection decides:
  - for a `QualifiedName`, a `FunctionSite`: `function` (`WireNodeId`,
    the function declaration's `function` node, FR-092), `declaration`
    (`OccurrenceKey`, that node's `declaration` occurrence) and
    `parameters`, `Vec<(Identifier, WireNodeId)>`, in declared order;
  - for an `OperationName`, an `OperationSite`: `anchor` and `frame`
    (`WireNodeId`), `frame_occurrence` (`OccurrenceKey`) and `clauses`
    (`Vec<ClauseSite>`, each `name: Identifier`, `node: WireNodeId` and
    `occurrence: OccurrenceKey`, in declaration order);
  - for a `ClauseName`, a `ClauseSite`: `name`, `node` and `occurrence`;
- or a typed `CallSiteRefusal` with no partial result.

## Behavior

- `call_site` SHALL compile `bytes` with the FR-278 composition
  against `packages` and `dependencies` under the default spine stage
  limits, and SHALL carry a compile refusal as the `CallSiteRefusal`
  variant for the input it concerns, each holding facade types and
  rendered messages:
  - a `model` declaration whose domain package the supplied `packages` do
    not admit: `ModelIntake`, with the declaration's alias;
  - the dependency input refused against the unit: `DependencyInput`, with
    its `DependencyInputRefusal`;
  - an `import` of the unit that does not resolve against `dependencies`:
    `Import`;
  - a supplied library's own refusal: `Dependency`, with the library path;
  - the unit's own source, forms, profile, assembly, check, link or emit
    refusal: `Compile`.
- For a function selection, `call_site` SHALL resolve the name by
  one-segment name lookup in the compiled package's declarations (OQ-5). A
  name with zero or more than one segment, or naming no declared function,
  SHALL refuse `CallSiteRefusal::UnknownFunction`, pairing the
  `QualifiedName` with the compiled package's own `package_id` -- never a
  bare `QualifiedName` (FR-088-AC-6, TC-258).
- For a function selection, `call_site` SHALL return the resolved
  function's own node id and its one `declaration` occurrence key
  (function node id, `declaration`, 0): the subject node id and occurrence
  key of the function's ADR-013 O-09 function-contract obligation
  identity, which CG computes and never derives itself.
- For a function selection, `call_site` SHALL pair each of the resolved
  function's declared parameters, in declared order, with its own node id,
  derived the same way `replay`'s own executor derives it (FR-098), so the
  same parameter always gets the same node id from either entry.
- For an operation selection, `call_site` SHALL resolve the model alias
  to its domain package, the object type in that package and the operation
  in the object type's effective view, exactly as FR-115's `Frame`
  selection resolves them, and SHALL return the operation's FR-105
  `operation_anchor` and `frame` node ids and the frame's `generated`
  occurrence key (FR-104-AC-5) -- the identities an FR-116 frame
  counterexample carries.
- For an operation selection, `call_site` SHALL return every `pre` and
  `post` clause of the unit whose operation resolves to the same
  (declaring object type, operation) as the selected frame, in declaration
  order, each with its declared name, its `state_clause` node id and its
  `claim` occurrence key (ADR-013 O-07).
- For a clause selection, `call_site` SHALL resolve the name among the
  unit's state clauses -- invariants, preconditions and postconditions --
  by declared name, and SHALL return the clause's declared name, its
  `state_clause` node id and its `claim` occurrence key, the identities a
  state-clause counterexample carries. A name that declares no state
  clause SHALL refuse `CallSiteRefusal::UnknownClause`, pairing the
  `ClauseName` with the compiled package's own `package_id` (FR-088-AC-6).
- An operation selection whose model alias or object type does not
  resolve, whose operation resolves to no single operation, or that no
  clause or attempt of the unit names (FR-105 emits no frame for it) SHALL
  refuse `CallSiteRefusal::UnknownOperation`, pairing the `OperationName`
  with the compiled package's own `package_id` (FR-088-AC-6).
- A broken invariant -- a resolved function's own node is not itself a
  function node, its parameter count disagrees with its checked signature,
  or a declared parameter or clause name is not itself a valid
  `Identifier` -- SHALL refuse `CallSiteRefusal::Fault(InternalFault)`.
- `CallSiteRefusal::code` SHALL return each refusal's code from the closed
  catalog `ReplayRefusal::code` (FR-098) draws on, the same code `replay`
  gives the same refusal: `Compile`, `ModelIntake`, `Import` and
  `Dependency` the code of the compile, intake, import-resolution or
  library refusal they carry, as `ReplayRefusal::Recompile` does;
  `DependencyInput` its `DependencyInputRefusal::code`; `UnknownFunction`,
  `UnknownOperation` and `UnknownClause` `missing_declaration`; and `Fault`
  `runtime_invariant`. Each variant other than `Fault` carries the code of
  the refusal it was built from rather than only its rendered message.
- The settlement of a refusal is keyed on timing. A `CallSiteRefusal` other
  than `Fault` refuses the obligation's own input before any backend run, so
  a consumer settles it `declined` (`TerminalValue::Declined`) and reports
  its code (`CallSiteRefusal::code`), by the terminal mapping ADR-013 C-09
  owns (QSpec FR-331). A `ReplayRefusal` after a backend run (the replay of
  a refutation) is no refusal of the obligation's input: it settles
  `TerminalValue::Inconclusive` with `InconclusiveCause::ReplayRefused`
  carrying its code, which `TerminalValue::from_replay_refusal` builds. A
  `Fault` of either, `CallSiteRefusal::Fault` or `ReplayRefusal::Fault`
  (and `ReplayRefusal::Admission` with `AdmissionFailure::Fault`), settles
  `failed` (`TerminalValue::Failed`).
- `call_site` SHALL read no path, environment variable, clock or search
  location, and SHALL give the same result for the same input.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-121-AC-1 | For a unit declaring one Boolean predicate `p(x: Int[0, 9]): Boolean`, `call_site` over `p` returns `parameters` holding exactly one pair, its `Identifier` equal to `x`, and its `WireNodeId` is the one `replay` accepts: a `ReplayRequestWire` keyed by that exact pair does not refuse `UnknownParameter` or `UnboundParameter`. | Test (TC-516) |
| FR-121-AC-2 | A function selection naming no declared function of the compiled package refuses `CallSiteRefusal::UnknownFunction`, pairing the `QualifiedName` with the compiled package's own `package_id`, never a bare `QualifiedName`. | Test (TC-516) |
| FR-121-AC-3 | For FR-108's ConfigVersion unit with its domain package supplied, an operation selection of `Config::ConfigVersion::attemptUpdate` returns the compiled package's `package_id`, the unit's one `operation_anchor` and one `frame` node id, a `frame_occurrence` equal to the key built through the facade as `OccurrenceKey::new(frame, Origin::new(Role::new("generated"), 0))`, and exactly one clause: `VersionUnchanged`, at its `postcondition` `state_clause` node and that node's `claim` occurrence at ordinal 0. Neither invariant of the unit is returned. | Test (TC-516) |
| FR-121-AC-4 | For a unit importing a library, `call_site` with a `DependencyInput` supplying that library returns a `package_id` and parameter pair such that a `ReplayRequestWire` carrying the library as its `dependencies` entry, keyed only by that `package_id` and pair, replays and agrees; with no dependency input the same unit refuses `CallSiteRefusal::Import`. | Test (TC-516) |
| FR-121-AC-5 | An operation selection whose model alias, object type or operation does not resolve refuses `CallSiteRefusal::UnknownOperation`, pairing the `OperationName` with the compiled package's own `package_id`. | Test (TC-516) |
| FR-121-AC-6 | For the ConfigVersion unit over a domain package also declaring `probe`, with a precondition `AttemptPre` on `attemptUpdate` declared before `VersionUnchanged` and a postcondition `ProbeUnchanged` on `probe`, the `attemptUpdate` selection's `clauses` are exactly `AttemptPre` then `VersionUnchanged`, and the `probe` selection's are exactly `ProbeUnchanged`, at a different `frame` node. | Test (TC-516) |
| FR-121-AC-7 | For the same domain package with no clause of the unit naming `probe`, the `Config::ConfigVersion::probe` selection refuses `CallSiteRefusal::UnknownOperation`, pairing the `OperationName` with the compiled package's own `package_id`. | Test (TC-516) |
| FR-121-AC-8 | For the AC-6 unit, a clause selection of `ParentOrder`, `NoCycle`, `AttemptPre` and `VersionUnchanged` each returns the compiled package's `package_id` and the clause's own `state_clause` node -- the two invariant nodes for the invariants, the precondition node for `AttemptPre`, a postcondition node for `VersionUnchanged` equal to the `attemptUpdate` selection's entry -- at `OccurrenceKey::new(node, Origin::new(Role::new("claim"), 0))`. `Absent` and the function name `sameIdentity` each refuse `CallSiteRefusal::UnknownClause`, pairing the `ClauseName` with the package. | Test (TC-516) |
| FR-121-AC-9 | The AC-6 unit with no domain package supplied refuses `CallSiteRefusal::ModelIntake` with the alias `Config`. | Test (TC-516) |
| FR-121-AC-10 | For the AC-4 unit, a `DependencyInput` supplying the imported library from a source with the unit's own authority and identity refuses `CallSiteRefusal::DependencyInput`, carrying `DependencyInputRefusal::SharedOwner` with `first` the unit, `second` the library and that shared authority and identity. | Test (TC-516) |
| FR-121-AC-11 | For the AC-4 unit, a `DependencyInput` supplying the imported library from source bytes that do not parse refuses `CallSiteRefusal::Dependency`, its `path` exactly that library's identity. | Test (TC-516) |
| FR-121-AC-12 | A `CallSite`'s `package` equals the S4 emitter's bytes for the same compile, and the RFC 8785 bytes of its `identity_preimage` member digest, under `quire.package.semantic/v2`, to the `CallSite`'s `package_id`. | Test (TC-516) |
| FR-121-AC-13 | From outside the crate, a `DeclaredDomain` over an integer range on a parameter node `call_site` returned is built through `qsl_replay`'s root paths alone (`ProofBound`, `DomainKey`, `FiniteBound`, `Integer`), its kind is `FiniteBoundKind::IntegerRange` and its interval equals the `IntegerInterval` built there; an inverted range refuses `EmptyFiniteBound::InvertedIntegerRange` through `FiniteBound::integer_range` and `EmptyInterval` through `IntegerInterval::new`. | Test (TC-516) |
| FR-121-AC-14 | `CallSiteRefusal::code` returns `missing_declaration` for AC-2's `UnknownFunction`, AC-5's `UnknownOperation` and AC-8's `UnknownClause`; for AC-9's `ModelIntake`, AC-10's `DependencyInput`, AC-11's `Dependency` and AC-4's `Import`, the code `ReplayRefusal::code` returns when `replay` is given the same unit, packages and dependency input; and for a unit with a syntax error, `Compile` carrying the same code as the `ReplayRefusal::Recompile` `replay` returns for it. | Test (TC-516) |
| FR-121-AC-15 | Worked example. For a unit declaring `p(x: Int[0, 9]): Boolean { x < 5 }` and `q(x: Int[0, 9]): Boolean { x < 5 }`, the `FunctionSite` for `p` carries `function` equal to the compiled graph's `function` node whose `declaration` is `p`, and `declaration` equal to `OccurrenceKey::new(function, Origin::new(Role::new("declaration"), 0))`. With a CG obligation kind and `arguments` `[(x's parameter node id, [0, 9])]`, these are the members of `p`'s ADR-013 O-09 function-contract obligation preimage. `q`'s `FunctionSite` has the same `parameters` (one shared parameter node) and a different `function`, so the two obligations differ. Recompiling the unit with a comment and blank lines inserted before `p` gives `p` the same `function` and `declaration`. Over the AC-6 unit, `sameIdentity`'s `function` equals no `ClauseSite` `node` returned for `attemptUpdate`, `probe` or any AC-8 clause selection. | Test (TC-516) |
| FR-121-AC-16 | A `ReplayRefusal` that is no fault, such as `UnboundParameter`, maps by `TerminalValue::from_replay_refusal` to `TerminalValue::Inconclusive(InconclusiveCause::ReplayRefused(code))` with `code` equal to `ReplayRefusal::code` for it, and category `inconclusive`; `ReplayRefusal::Fault` and `ReplayRefusal::Admission` with `AdmissionFailure::Fault` map to `TerminalValue::Failed`. | Test (TC-516) |

## Dependencies

- [FR-027](FR-027-export-compiled-native-package.md): the spine compile
  `call_site` runs.
- [FR-088](FR-088-clause-name-and-type-identity.md): no identity treats a
  bare name as sufficient; `UnknownFunction` and `UnknownOperation` pair it
  with the package it was looked up in (FR-088-AC-6, TC-258).
- [FR-098](FR-098-execute-a-replay-request.md): the replay facade
  `call_site` is a second entry of, and the parameter node id derivation it
  shares.
- [FR-105](FR-105-emit-state-nodes.md): the `state_clause`,
  `operation_anchor` and `frame` nodes an operation selection names.
- [FR-115](FR-115-run-an-operation-frame-over-an-invocation.md): the
  typed `OperationName` resolution an operation selection shares.
- [FR-116](FR-116-replay-a-frame-counterexample.md): the frame
  counterexample whose anchor, frame and occurrence an `Operation`
  selection supplies.
- [ADR-011](../decisions/ADR-011-stage-dag-and-dependency-architecture.md)
  §6.1, §3 FB-05, T-12 rule (a).
- ADR-013 O-07, O-09 (the function-contract obligation identity whose
  subject members a `FunctionSite` carries), O-25, C-11, TK-01.
- [FR-092](FR-092-key-type-parameter-and-declared-nodes.md): the
  `function` node a `FunctionSite` names.
- ADR-015 D-1: the dependency input.

## Status

Implemented: `qsl_replay::call_site`, sharing its parameter node-key
derivation with `qsl_replay::replay`'s own selection
(`callable_parameter_keys`) and its operation resolution with FR-115's
`Frame` selection and its clause lookup with FR-106's clause selection,
and `CallSiteRefusal::code` giving each refusal the code `replay` gives
it, verified by TC-516, with `FunctionSite`'s `function` and `declaration`
members (AC-15) read from the same function node.
