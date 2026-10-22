---
id: FR-404
title: "Carry the call's plugin replay context through the replay facade"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-395
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-290
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-100
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-098
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-255
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-278
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-060
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/ADR-029
    type: traces_to
---
# FR-404: Carry the call's plugin replay context through the replay facade

## Description

A driver that runs one function through the replay facade
(`run_with_requirements`, FR-395) routes the call's requirement items to a
process provider. When the provider returns a counterexample, the driver
settles it through the item's S6a replay (FR-290 "Replay context", FR-098),
and the replay request's package-wide members come from a
`PluginReplayContext`. `PluginReplayContext::new` takes the `CheckedUnit`,
`EmittedUnit` and `AdmittedModels` of the compile and the run's
`CallerLimits`. Those units are `qsl_replay::spine` types, which a consumer
may not name (FR-060 T12-A), and `run_with_requirements` keeps none of them.
A driver that compiled the package a second time to get them could build a
context from a compile other than the one its items came from, or under other
limits.

This requirement adds the facade entry that runs the call as
`run_with_requirements` does and returns, from that same compile, a replay
context bound to the run: it carries the run's `package_id` and the call's
members, and it builds requests only for the call's own items. The driver
names no spine type, builds no context of its own, and cannot pair one run's
items with another run's context.

## Terms

- **Run**: one call of `run_with_replay_context`: one parse, one `select`,
  one `check`, one `package` and one call of the selected function.
- **Call replay context**: the context the entry returns with a run
  (`CallReplayContext`, design name). It holds a `PluginReplayContext` built
  from the run's units and limits, the run's `package_id`, and the occurrence
  keys of the run's `CallRequirements` members.
- **Item of the call**: as FR-395 defines it: a requirement item written from
  a record whose owner is the selected function. Its occurrence key is a
  member's key.

## Inputs

- The arguments of `run_with_requirements` (FR-395 Inputs), except that the
  limits are the run's `CallerLimits` (FR-255) instead of `SpineLimits`.
- For a request: an occurrence key, the substituted `ProofBound`s of the
  item's extent, the plugin's `BackendId` and the `CanonicalAssignment`s that
  `read_plugin_assignments` returned (FR-290).

## Outputs

- `run_with_replay_context` (design name; the implementing ticket chooses the
  Rust spelling) returns, on success, `RunWithReplayContext`: the run's
  `package_id`, its `CallOutcome` and its `CallRequirements` (each as
  `run_with_requirements` returns it for the same inputs, with the
  `CallerLimits`' `spine` limits as its `SpineLimits`), and the run's
  `CallReplayContext`.
- `CallReplayContext::package_id` returns the run's `package_id`.
- `CallReplayContext::request(occurrence, proof_bounds, plugin, assignments)`
  returns a `ReplayRequestWire` or a `PluginReplayRefusal` (FR-290). This
  requirement adds no refusal variant.
- The entry's refusal type is `RunRefusal` (FR-100), which this requirement
  extends by no variant.

## Behavior

- **One compile.** The entry SHALL compile the source once, and the outcome,
  the `package_id`, the members and the context SHALL all come from that
  compile. The context's `PluginReplayContext` SHALL be the one FR-290 builds
  from the run's own `CheckedUnit`, the `EmittedUnit` that `package` emitted
  from it, the run's `AdmittedModels` and the input `CallerLimits`.
- **Compile limits.** The entry SHALL run `parse`, `select` and `check` under
  the `CallerLimits`' `spine` limits and `package` under its `library`
  limits, so the context's `stage_limits` are the limits the package was
  compiled under (FR-290 replay-context table, FR-278). The call charges
  under the `Call`'s own `accounting` as FR-100 states. The context's
  `accounting_limits` are the `CallerLimits`' `accounting`, the budgets a
  replay of a counterexample runs under.
- **Package binding.** `CallReplayContext::package_id` and the `package_id`
  member of every request the context builds SHALL equal the run's
  `package_id`, and every request's `source_digests`, `dependencies` and
  `byte_provision` SHALL be those of the run's compile.
- **Occurrence binding.** `CallReplayContext::request` SHALL build a request
  only for an occurrence key that is the key of a member of the run's
  `CallRequirements`. For any other key it SHALL refuse
  `PluginReplayRefusal::UnknownOccurrence` (`missing_declaration`/
  `missing-name`), retaining the key, and build no request. This covers a
  record of the same package that belongs to another function, the
  `declaration` occurrence of any function (QSL's request writer writes no
  item there, FR-290), and a key the package does not hold.
- **Refusal order.** `CallReplayContext::request` SHALL check the occurrence
  binding first, and then FR-290's steps 2 to 4 in their order
  (`NotReplayable`, `UnresolvedDependency`, `DuplicateBoundKey`, and `Fault`
  for a broken invariant), returning the first refusal.
- **Same request.** For a key that passes the occurrence binding, the
  request or refusal SHALL equal what `PluginReplayContext::request` returns
  for the same arguments over a context built by `PluginReplayContext::new`
  from the units of a compile of the same inputs under the same
  `CallerLimits`. The request's `selected_function` is the selected call's
  function, because the item's owner is that function (FR-395).
- **Outcome independence.** The entry SHALL return the context of a call
  whose outcome is `Refused`, `Undefined` or `Incomplete`, so long as the call
  ran, as FR-395 returns its members.
- **Typed refusals.** The entry SHALL refuse exactly as
  `run_with_requirements` refuses for the same inputs (FR-395 "Typed
  refusals" and "Broken ownership is a fault"), and SHALL return no context
  and no `CallRequirements` with a refusal.
- **No consumer construction.** `CallReplayContext` SHALL have no public
  constructor, no public field and no deserializer, so a consumer obtains one
  only from the entry's result. `PluginReplayContext::new` keeps its FR-290
  signature; its inputs stay spine types, which a consumer does not name.
- **Facade boundary.** The `qsl_replay` root SHALL expose
  `run_with_replay_context`, `RunWithReplayContext` and `CallReplayContext`,
  and SHALL re-export every type of the entry's and of `request`'s signature
  and result that it does not export, so that a consumer runs a call, builds
  a request and settles its refusal without naming `qsl_replay::spine` or
  another QSL crate: among them `CallerLimits`, `OccurrenceKey`,
  `ProofBound`, `BackendId`, `CanonicalAssignment`, `ReplayRequestWire` and
  `PluginReplayRefusal`. `qsl_replay::spine` stays internal (FR-060 T12-A).
  `run_with_requirements` keeps its signature and result (FR-395).

## Acceptance Criteria

Fixture **R** is one `1-draft` unit under profile alias `v`
(`quire.value.complete/v1`) with the functions of FR-290-AC-12 and AC-15:

- `function p using v(x: Int[0, 9]): Boolean pure { x > 3 }`;
- `function q using v(x: Int[0, 9]): Integer pure { x + 1 }`;
- `function r using v(x: Int[0, 9]): Integer pure { 10 mod x }`.

Fixture **R2** is R with `function s using v(x: Int[0, 9]): Integer pure { x + 2 }`
appended. The plugin is `BackendId` `acme`. Unless an AC says otherwise, the
limits are `CallerLimits::default()` and the proof bounds are those of the
item that `items_from_requirements` writes for the member.

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-404-AC-1 | Over R, `q` with `x = 3` returns outcome `completed` `4`, one member, and a context whose `package_id` equals the result's; the `package_id`, outcome and members equal those `run_with_requirements` returns for the same inputs with the default `CallerLimits`' `spine` limits, and the `package_id` equals `compile_package`'s over the same source. | Test (TC-1173) |
| FR-404-AC-2 | Over AC-1's run, `request` for `q`'s member with assignment `x = 9` returns a request equal, member by member, to what `PluginReplayContext::request` returns for the same arguments over a context built by `PluginReplayContext::new` from a separate `parse`, `select`, `check` and `package` of R under the same limits. Its `package_id` is the run's, its `selected_function` is `q` and its `occurrence_key` is the member's. `replay` of it under the `CallerLimits`' `replay` limits completes `10`, and the item settles `inconclusive` with cause `replay-parity`, naming `acme`. | Test (TC-1173) |
| FR-404-AC-3 | Over R, `r` with `x = 0` returns outcome `undefined` with one member and a context. `request` for that member with assignment `x = 0` gives a request whose `selected_function` is `r` and whose `obligation_identity` is the `process-provider` identity of the member's node and occurrence; `replay` finds the application undefined, and `TerminalRecord::from_plugin_counterexample` settles `refuted`, naming `acme`. `q` with `x = 3` under call accounting `work_units = 0` returns outcome `incomplete` on `work_units` and a context whose request for `q`'s member equals AC-2's. | Test (TC-1173) |
| FR-404-AC-4 | The context of AC-1's run refuses `UnknownOccurrence` (`missing_declaration`/`missing-name`), retaining the key, for `r`'s member key (a record of R owned by `r`), for `p`'s `declaration` occurrence, and for the key with node `11` repeated 32 times and occurrence `{"ordinal":0,"role":"declaration"}`. For `p`'s `declaration` occurrence with assignment `x = 0`, `PluginReplayContext::request` over AC-2's separately built context builds a request. Each refusal settles the item `failed`, naming `acme`. | Test (TC-1173) |
| FR-404-AC-5 | Over R and R2, the runs of `q` with `x = 3` return different `package_id`s, and each context's request for its own `q` member carries its own run's `package_id` and source digest. R2's `s` with `x = 3` returns one member, and the context of R's `q` run refuses `UnknownOccurrence` for that member's key. | Test (TC-1173) |
| FR-404-AC-6 | Over R, `q` with `x = 3` under `CallerLimits` whose `s1.tokens` is 200000, whose `library.definitions` is 9000 and whose accounting `work_units` is 5000 completes, and the request for its member holds `s1.tokens` 200000 and `library.definitions` 9000 in `stage_limits` and `work_units` 5000 in `accounting_limits`. The same call under `CallerLimits` whose `s1.tokens` is 1 refuses `Compile` with no context. | Test (TC-1173) |
| FR-404-AC-7 | The context of AC-1's run refuses a key that passes no binding and carries two proof bounds at one `DomainKey` with `UnknownOccurrence`; `q`'s member with two proof bounds at one `DomainKey` refuses `DuplicateBoundKey` (`invalid_runtime_input`/`duplicate-member`). | Test (TC-1173) |
| FR-404-AC-8 | Over R, the function `nope` refuses `MissingDeclaration`, `q` with a Boolean `x` refuses `WrongValueKind` at position 0, a source that does not parse refuses `Compile`, and a cancelled handle refuses `Cancelled`; each refusal equals `run_with_requirements`' for the same inputs and returns no context and no members. | Test (TC-1173) |
| FR-404-AC-9 | A consumer runs AC-1's call, builds AC-2's request and settles AC-4's refusal through the `qsl_replay` root alone, without naming `qsl_replay::spine`, `qsl_foundation` or `qsl_semantics`; FR-060's T12-A rule reports no call site outside its allowed callers for them. A consumer that names a `CallReplayContext` struct literal or a constructor does not compile. | Test (TC-1173) |

## Dependencies

- FR-395 (the facade's call and members), FR-290 (`PluginReplayContext`,
  its request table and refusals), FR-100 (`run`, `Call`, `RunRefusal`),
  FR-098 (`replay`), FR-255 (`CallerLimits`), FR-278 (the library operations
  and their limits), FR-060 (the T12-A boundary), ADR-011, ADR-029.
