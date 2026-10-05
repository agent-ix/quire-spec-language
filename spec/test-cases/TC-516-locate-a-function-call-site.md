---
id: TC-516
title: "call_site names a function's parameters and an operation's and a state clause's identities, matching what replay accepts"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-121
    type: verifies
---
# TC-516: call_site names a function's parameters and an operation's and a state clause's identities, matching what replay accepts

## Description

Verify `qsl_replay::call_site` over real complete-V1 source: it pairs a
named function's declared parameters with their node ids, and the pairing
is the one `qsl_replay::replay` actually accepts, proved by driving `replay`
with a request keyed by `call_site`'s own pairs rather than by re-running
the same node lookup `call_site` uses and comparing -- including for a unit
compiled against a dependency input. It locates a named operation's anchor,
frame and state clause identities in a unit compiled against a domain
package, read back against the compiled graph's nodes by semantic form and
against occurrence keys built through the facade's own `OccurrenceKey`,
`Origin` and `Role`, keeping only the selected operation's clauses in
declaration order, and locates a named state clause of any kind. It
refuses a function, operation or clause resolving to nothing, pairing it
with the package it was looked up in, and an unsupplied domain package as
`ModelIntake`, and a refused dependency input or an uncompilable supplied
library as `DependencyInput` or `Dependency`.

It returns the compiled package's bytes its `package_id` names, builds a
`DeclaredDomain` through the facade's re-exports, and keys a named state
field under its declaring type's node by its name ordinal, apart from any
population key, and keys a named population as the requirement records do.

Scope: FR-121-AC-1 to FR-121-AC-22.

## Test Procedure

1. Compile a unit declaring one Boolean predicate `p(x: Int[0, 9]): Boolean
   { x < 5 }` through `call_site`. Assert `parameters` holds exactly one
   pair, whose `Identifier` is `x`. Build a `ReplayRequestWire` for the same
   source and function, keyed by that exact `(Identifier, WireNodeId)` pair
   (the `WireNodeId` as the `CanonicalAssignment::parameter`), and call
   `replay`.
2. Compile the TC-452 fixture unit `F` (`function f(x: Int[0, 9]): Integer`)
   through `call_site` naming a function the compiled package does not
   declare.
3. Compile FR-108's ConfigVersion unit through `call_site` with the
   `test/config-version` domain package document, selecting
   `Config::ConfigVersion::attemptUpdate`. Compare the answer with the
   compiled package's `package_id`, the graph's one `state`/`operation_anchor`
   and one `state`/`frame` node, and its one postcondition `state_clause`
   node; build the expected occurrence keys with
   `OccurrenceKey::new(node, Origin::new(Role::new(role), 0))` from the
   facade's re-exports.
4. Compile TC-444 step 7's unit importing `test/units` through `call_site`
   selecting `q`, once with a `DependencyInput` supplying `test/units` and
   once with none. From the first answer alone, key a `ReplayRequestWire`
   carrying `test/units` as its `dependencies` entry (its `package_id` and
   `q`'s one parameter), and call `replay` with the input `x = 3`.
5. Compile the step 3 unit through `call_site` selecting
   `Nope::ConfigVersion::attemptUpdate`, `Config::Nope::attemptUpdate` and
   `Config::ConfigVersion::nope`.
6. Build the two-operation unit: the step 3 domain package with `probe`
   added, the step 3 invariants, a precondition `AttemptPre` on
   `attemptUpdate`, then `VersionUnchanged`, then a postcondition
   `ProbeUnchanged` on `probe`, and the function `sameIdentity`. Select
   `attemptUpdate` and `probe`.
7. Build the same unit with no `AttemptPre` and no `ProbeUnchanged`, and
   select `Config::ConfigVersion::probe`.
8. Over the step 6 unit, select the clauses `ParentOrder`, `NoCycle`,
   `AttemptPre`, `VersionUnchanged`, `Absent` and `sameIdentity`. Compare
   the answers with the graph's `state_clause` nodes of each kind, with the
   step 6 `attemptUpdate` answer, and with `claim` occurrence keys built
   through the facade.
9. Compile the step 6 unit through `call_site` with no domain package.
10. Compile the step 4 unit through `call_site` with `test/units` supplied
    from a source with the unit's own authority and identity.
11. Compile the step 4 unit through `call_site` with `test/units` supplied
    from source bytes that do not parse.
12. Compile the step 2 unit through `call_site`; digest the RFC 8785 bytes
    of the returned `package`'s `identity_preimage` member.
13. Over the step 1 unit, from outside the crate, build a `DeclaredDomain`
    for `[0, 9]` on `x`'s node using only `qsl_replay` root paths, and an
    inverted range through `FiniteBound::integer_range` and
    `IntegerInterval::new`.
14. Read `CallSiteRefusal::code` for the refusals of steps 2, 5, 8, 9,
    10 and 11 and step 4's unit with no dependency input, and for a unit
    with a syntax error; run `replay` over the same unit, packages and
    dependency input for each and read `ReplayRefusal::code`.

15. Compile a unit declaring `p(x: Int[0, 9]): Boolean { x < 5 }` and
    `q(x: Int[0, 9]): Boolean { x < 5 }`, select `p` and `q`, and compare
    each `function` and `declaration` with the compiled graph's `function`
    nodes and an occurrence key built through the facade. Recompile the unit
    with a comment and blank lines inserted before `p` and select `p`
    again. Over the step 6 unit, select `sameIdentity` and compare its
    `function` with every `ClauseSite` `node` of steps 6 and 8.
16. Map `ReplayRefusal::UnboundParameter` and the two fault refusals
    (`ReplayRefusal::Fault`, `ReplayRefusal::Admission` carrying
    `AdmissionFailure::Fault`) through `TerminalValue::from_replay_refusal`.
17. Map a `CallSiteRefusal::Compile`, a `CallSiteRefusal::UnknownFunction` and
    a `CallSiteRefusal::Fault` through `TerminalValue::from_call_site_refusal`.
18. Add `Sub`, a subtype of `ConfigVersion` declaring `zeta` then `alpha`,
    to the step 3 domain package, and an invariant on `Config::Sub` to the
    step 3 unit. Select `parent` and `versionNumber` through
    `ConfigVersion`, and `versionNumber`, `alpha` and `zeta` through `Sub`;
    compare each node with the graph's own `object_type` node for its
    declaring type. Select `Config::Sub.nope`, `Config::ConfigVersion.alpha`,
    `Config::Nope.parent` and `Nope::Sub.alpha`.
19. Over the step 18 domain package without the `Sub` invariant, select
    `Config::Sub.alpha`.
20. Over the step 3 unit, select `Config::ConfigVersion.parent`, and read the
    `config_history` population key from the compiled package's requirement
    records.
21. Add a second population `aaa_subs` of `Sub`, after `config_history`, to
    the step 18 domain package. Over the step 3 unit's clauses, select
    `Config::config_history` and `Config::aaa_subs`, and read the
    population keys the requirement records carry.
22. Over the step 21 unit, select `Config::nope`, `Nope::config_history`
    and `Config::ConfigVersion`.

Tag the tests `#[trace("TC-516", "FR-121-AC-n")]`.

## Expected Results

- Step 1: `replay` does not refuse `UnknownParameter` or `UnboundParameter`
  -- the node id `call_site` names for `x` is the one `replay`'s own
  argument join accepts.
- Step 2: `call_site` refuses `CallSiteRefusal::UnknownFunction`, pairing
  the given `QualifiedName` with the compiled package's own `package_id`,
  never a bare `QualifiedName`.
- Step 3: `package_id`, `anchor` and `frame` equal the compiled ones;
  `frame_occurrence` is the frame node's `generated` occurrence at ordinal
  0; `clauses` holds exactly `VersionUnchanged`, at the postcondition node
  and its `claim` occurrence at ordinal 0. `ParentOrder` and `NoCycle` are
  absent.
- Step 4: with the dependency input, the parameter is `x` and the replay
  reproduces without a witness with the value `false`; without it,
  `call_site` refuses `CallSiteRefusal::Import`.
- Step 5: each refuses `CallSiteRefusal::UnknownOperation`, pairing the
  given `OperationName` with the compiled package's own `package_id`.
- Step 6: `attemptUpdate`'s clauses are exactly `AttemptPre`, then
  `VersionUnchanged`; `probe`'s are exactly `ProbeUnchanged`, at a
  different frame node.
- Step 7: `call_site` refuses `CallSiteRefusal::UnknownOperation`, pairing
  the `OperationName` with the compiled package's own `package_id`.
- Step 8: the invariants' answers are the two invariant nodes,
  `AttemptPre`'s the precondition node, `VersionUnchanged`'s a
  postcondition node equal to the `attemptUpdate` answer's entry, each at
  its `claim` occurrence at ordinal 0 and with the compiled `package_id`;
  `Absent` and `sameIdentity` each refuse `CallSiteRefusal::UnknownClause`
  paired with the package.
- Step 9: `call_site` refuses `CallSiteRefusal::ModelIntake` with the alias
  `Config`.
- Step 10: `call_site` refuses `CallSiteRefusal::DependencyInput` with
  `DependencyInputRefusal::SharedOwner`: `first` the unit, `second`
  `test/units`, and the unit's authority and identity.
- Step 11: `call_site` refuses `CallSiteRefusal::Dependency` whose `path`
  is exactly `test/units`.
- Step 12: `package` equals the emitter's bytes, and the digest under
  `quire.package.semantic/v2` equals `package_id`.
- Step 13: the domain names `x`'s node and the range, of kind
  `FiniteBoundKind::IntegerRange`; the inverted range refuses
  `EmptyFiniteBound::InvertedIntegerRange` and `EmptyInterval`.
- Step 14: `missing_declaration` for steps 2, 5 and 8; for the others,
  each `CallSiteRefusal::code` equals the `ReplayRefusal::code` for the
  same input, the syntax error's equal to that of `replay`'s
  `ReplayRefusal::Recompile`.
- Step 15: `p`'s `function` is the graph's `function` node declared `p`
  and its `declaration` is that node's `declaration` occurrence at ordinal
  0; `q`'s `parameters` equal `p`'s and its `function` differs; the
  recompiled unit gives `p` the same `function` and `declaration`;
  `sameIdentity`'s `function` equals no `ClauseSite` `node`.
- Step 16: `UnboundParameter` maps to `Inconclusive(ReplayRefused(code))` with
  `ReplayRefusal::code`'s code and category `inconclusive`; both faults map
  to `Failed`.
- Step 17: the `Compile` and `UnknownFunction` refusals map to `Declined` with
  cause `InvalidInput` and each refusal's own code as `DeclineCode::Qsl` (category `refusal`); the
  `Fault` maps to `Failed`.
- Step 18: `parent` is `[0]` and `versionNumber` `[1]` under
  `ConfigVersion`'s node, through either type; `alpha` is `[0]` and `zeta`
  `[1]` under `Sub`'s node; the four unresolved selections refuse
  `CallSiteRefusal::UnknownField` paired with the package.
- Step 19: the graph holds no `Sub` node, and the key equals step 18's.
- Step 20: the population key is `DomainKey::Population` with member type
  `ConfigVersion`'s node and ordinal 0, and differs from `parent`'s
  `DomainKey::Node` on that node with path `[0]`.
- Step 21: `config_history` is `DomainKey::Population` on `ConfigVersion`'s
  node at ordinal 1, the records' only population key; `aaa_subs` is on
  `Sub`'s node at ordinal 0.
- Step 22: each refuses `CallSiteRefusal::UnknownPopulation` paired with
  the package.
