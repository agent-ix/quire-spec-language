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
  - target: ix://agent-ix/quire-spec-language/FR-075
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-013
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
context bound to the run. The context names an item by FR-395's pairing key,
(`package_id`, request index), and builds a request only for an item of its
own run's call. An occurrence key alone cannot name the run: a node's key is
derived from the node's content and carries no package (ADR-013 O-04), so two
packages that hold the same function hold equal keys for its records. The
driver names no spine type, builds no context of its own, and an item of
another run's package is refused.

## Terms

- **Run**: one call of `run_with_replay_context`: one parse, one `select`,
  one `check`, one `package` and one call of the selected function.
- **Call replay context**: the context the entry returns with a run
  (`CallReplayContext`, design name). It holds a `PluginReplayContext` built
  from the run's units and limits, the run's `package_id`, and the run's
  `CallRequirements` members.
- **Item of the call**: as FR-395 defines it: a requirement item written from
  a record whose owner is the selected function. Its pairing key is the run's
  `package_id` and a member's request index.

## Inputs

- The arguments of `run_with_requirements` (FR-395 Inputs), except that the
  limits are the run's `CallerLimits` (FR-255) instead of `SpineLimits`.
- For a request: the item's pairing key (the `package_id` of the compile that
  wrote the item and the item's request index), the substituted `ProofBound`s
  of the item's extent, the plugin's `BackendId` and the
  `CanonicalAssignment`s that `read_plugin_assignments` returned (FR-290).
  The driver writes the item with `items_from_requirements` (FR-075) from the
  run's records, as it does for `run_with_requirements`.

## Outputs

- `run_with_replay_context` (design name; the implementing ticket chooses the
  Rust spelling) returns, on success, `RunWithReplayContext`: the run's
  `package_id`, its `CallOutcome` and its `CallRequirements`, and the run's
  `CallReplayContext`.
- `CallReplayContext::package_id` returns the run's `package_id`.
- `CallReplayContext::request(package_id, request_index, proof_bounds,
  plugin, assignments)` returns a `ReplayRequestWire` or a
  `PluginReplayRefusal` (FR-290). This requirement adds two variants to
  `PluginReplayRefusal`:
  - `ForeignRun`, `stale_dependency`/`content-mismatch`, retaining the
    received `package_id` and the run's: the code FR-098 gives a request whose
    `package_id` is not the recompiled one;
  - `NotACallItem`, `missing_declaration`/`missing-name`, retaining the
    request index: the code FR-290 gives an occurrence the package does not
    hold.
  Each settles the item `failed` through `PluginReplayRefusal::record`, with
  its diagnostic in the outcome document (FR-290).
- The entry's refusal type is `RunRefusal` (FR-100), which this requirement
  extends by no variant.

## Behavior

- **One compile.** The entry SHALL compile the source once, and the outcome,
  the `package_id`, the members and the context SHALL all come from that
  compile. The context's `PluginReplayContext` SHALL be the one FR-290 builds
  from the run's own `CheckedUnit`, the `EmittedUnit` that `package` emitted
  from it, the run's `AdmittedModels` and the input `CallerLimits`.
- **Limits mapping.** The entry's `SpineLimits` are the `CallerLimits`'
  `spine` field. With them the entry SHALL run `parse` (`source`), `select`
  (`model`) and `check` (`checking`, `dependencies`, `assembly`, `imports`),
  as `run_with_requirements` runs them for that `SpineLimits`. `package`
  takes the field-less spine `PackageLimits`, which bounds nothing (FR-278). The call charges under the `Call`'s own
  `accounting`, as FR-100 states. The other `CallerLimits` fields
  (`library`, `admission`, `observation`, `explore`, `replay`, `format`,
  `accounting`) bound no stage of the run: the context copies them into each
  request (FR-290 replay-context table), where `accounting` becomes the
  request's `accounting_limits`, and FR-098's recompile of the request runs
  under the request's spine settings.
- **Package binding.** `CallReplayContext::package_id` and the `package_id`
  member of every request the context builds SHALL equal the run's
  `package_id`, and every request's `source_digests`, `dependencies` and
  `byte_provision` SHALL be those of the run's compile.
- **Item binding.** `CallReplayContext::request` SHALL build a request only
  for a pairing key whose `package_id` equals the run's and whose request
  index is a member's, at that member's occurrence key. It SHALL refuse
  `ForeignRun` when the `package_id` differs from the run's, and
  `NotACallItem` when the request index is no member's: a record of the
  package that belongs to another function, or an index past the package's
  records. Neither builds a request.
- **Refusal order.** `CallReplayContext::request` SHALL check `ForeignRun`,
  then `NotACallItem`, and then FR-290's steps 2 to 4 at the member's
  occurrence key in their order (`NotReplayable`, `UnresolvedDependency`,
  `DuplicateBoundKey`, and `Fault` for a broken invariant), returning the
  first refusal. FR-290's step 1 cannot fail there, because a member's key is
  a record of the package.
- **Same request.** For a pairing key that passes the item binding, the
  request or refusal SHALL equal what `PluginReplayContext::request` returns
  for the member's occurrence key and the same other arguments, over a
  context built by `PluginReplayContext::new` from the units of a compile of
  the same inputs under the same `CallerLimits`. The request's
  `selected_function` is the selected call's function, because the item's
  owner is that function (FR-395).
- **Outcome independence.** The entry SHALL return the context of a call
  whose outcome is `Refused`, `Undefined` or `Incomplete`, so long as the call
  ran, as FR-395 returns its members.
- **Typed refusals.** The entry SHALL refuse exactly as
  `run_with_requirements` refuses for the same inputs with the
  `CallerLimits`' `spine` field as its `SpineLimits` (FR-395 "Typed
  refusals" and "Broken ownership is a fault"), and SHALL return no context
  and no `CallRequirements` with a refusal.
- **No consumer construction.** `CallReplayContext` SHALL have no public
  constructor (no `new`, no `Default`, no `From` or `TryFrom`), no public
  field and no `Deserialize` implementation, so a consumer obtains one only
  from the entry's result. `PluginReplayContext::new` keeps its FR-290
  signature; its inputs stay spine types, which a consumer does not name.
- **No unbound access.** `CallReplayContext` SHALL expose only `package_id`
  and `request`. It SHALL implement no `Deref`, `AsRef`, `Borrow` or `Into`
  to `PluginReplayContext` and have no method that returns the wrapped
  context, so a consumer cannot reach the unbound
  `PluginReplayContext::request` through it.
- **Facade boundary.** The `qsl_replay` root SHALL expose
  `run_with_replay_context`, `RunWithReplayContext` and `CallReplayContext`,
  and SHALL re-export every type of the entry's and of `request`'s signature
  and result that it does not export, so that a consumer runs a call, builds
  a request and settles its refusal without naming `qsl_replay::spine`,
  `qsl_foundation` or `qsl_semantics`: among them `CallerLimits`,
  `RequestIndex`, `ProofBound`, `BackendId`, `CanonicalAssignment`,
  `ReplayRequestWire` and `PluginReplayRefusal`. The consumer writes the
  call's items with `qsl_route::items_from_requirements` (FR-075), which is
  not part of this facade. `qsl_replay::spine` stays internal (FR-060 T12-A).
  `run_with_requirements` keeps its signature and result (FR-395).

## Acceptance Criteria

Fixture **R** is one `1-draft` unit under profile alias `v`
(`quire.value.complete/v1`) with the functions of FR-290-AC-12 and AC-15:

- `function p using v(x: Int[0, 9]): Boolean pure { x > 3 }`;
- `function q using v(x: Int[0, 9]): Integer pure { x + 1 }`;
- `function r using v(x: Int[0, 9]): Integer pure { 10 mod x }`.

Fixture **R2** is R with `function s using v(x: Int[0, 9]): Integer pure { x + 2 }`
appended. The plugin is `BackendId` `acme`. Unless an AC says otherwise, the
limits are `CallerLimits::default()`, a request names its member by the run's
`package_id` and the member's request index, and the proof bounds are those
of the item that `items_from_requirements` writes for the member.

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-404-AC-1 | Over R, `q` with `x = 3` returns outcome `completed` `4`, one member, and a context whose `package_id` equals the result's; the `package_id`, outcome and members equal those `run_with_requirements` returns for the same inputs with `CallerLimits::default().spine` as its `SpineLimits`, and the `package_id` equals `compile_package`'s over the same source. | Test (TC-1173) |
| FR-404-AC-2 | Over AC-1's run, `request` for `q`'s member with assignment `x = 9` returns a request equal, member by member, to what `PluginReplayContext::request` returns for the member's occurrence key and the same other arguments over a context built by `PluginReplayContext::new` from a separate `parse`, `select`, `check` and `package` of R under the same limits. Its `package_id` is the run's, its `selected_function` is `q` and its `occurrence_key` is the member's. `replay` of it under the `CallerLimits`' `replay` limits completes `10`, and the item settles `inconclusive` with cause `replay-parity`, naming `acme`. | Test (TC-1173) |
| FR-404-AC-3 | Over R, `r` with `x = 0` returns outcome `undefined` with one member and a context. `request` for that member with assignment `x = 0` gives a request whose `selected_function` is `r` and whose `obligation_identity` is the `process-provider` identity of the member's node and occurrence; `replay` finds the application undefined, and `TerminalRecord::from_plugin_counterexample` settles `refuted`, naming `acme`. `q` with `x = 3` under call accounting `work_units = 0` returns outcome `incomplete` on `work_units` and a context whose request for `q`'s member with assignment `x = 9` equals AC-2's. | Test (TC-1173) |
| FR-404-AC-4 | The context of AC-1's run refuses `NotACallItem` (`missing_declaration`/`missing-name`), retaining the index, for the run's `package_id` with `r`'s member's request index (a record of R owned by `r`) and with request index 99; each refusal settles the item `failed`, naming `acme`. | Test (TC-1173) |
| FR-404-AC-5 | Over R and R2, the runs of `q` with `x = 3` return different `package_id`s and members with equal occurrence keys. The context of R's `q` run refuses `ForeignRun` (`stale_dependency`/`content-mismatch`), retaining both `package_id`s, for the pairing key of R2's `q` member, and builds no request; each context's request for its own `q` member carries its own run's `package_id` and source digest. The refusal settles the item `failed`, naming `acme`. | Test (TC-1173) |
| FR-404-AC-6 | Over R, `q` with `x = 3` under `CallerLimits` whose `s1.tokens` is 200000, whose `library.definitions` is 9000 and whose accounting `work_units` is 5000 completes, and the request for its member holds `s1.tokens` 200000 and `library.definitions` 9000 in `stage_limits` and `work_units` 5000 in `accounting_limits`. The same call under `CallerLimits` whose `s1.tokens` is 1 refuses `Compile` at the S1 token limit with no context, while `run_with_requirements` over the same inputs with `SpineLimits::default()` completes. | Test (TC-1173) |
| FR-404-AC-7 | For AC-1's run, each with two proof bounds at one `DomainKey`: R2's `q` pairing key refuses `ForeignRun`; the run's `package_id` with `r`'s member's index refuses `NotACallItem`; `q`'s member refuses `DuplicateBoundKey` (`invalid_runtime_input`/`duplicate-member`). | Test (TC-1173) |
| FR-404-AC-8 | Over R, the function `nope` refuses `MissingDeclaration`, `q` with a Boolean `x` refuses `WrongValueKind` at position 0, a source that does not parse refuses `Compile`, and a cancelled handle refuses `Cancelled`; each refusal equals `run_with_requirements`' for the same inputs with `CallerLimits::default().spine`, and returns no context and no members. | Test (TC-1173) |
| FR-404-AC-9 | A consumer test runs AC-1's call, builds AC-2's request and settles AC-4's refusal naming only `qsl_replay` root paths and `qsl_route::items_from_requirements`, and no path through `qsl_replay::spine`, `qsl_foundation` or `qsl_semantics`; FR-060's T12-A rule reports no call site outside its allowed callers for it. Each of these does not compile: a `CallReplayContext` struct literal; `CallReplayContext::new()`; `CallReplayContext::default()`; `serde_json::from_str::<CallReplayContext>("{}")`; `let _: &PluginReplayContext = &*context;`; `let _: &PluginReplayContext = context.as_ref();`. | Test (TC-1173) |

## Dependencies

- FR-395 (the facade's call, members and pairing key), FR-290
  (`PluginReplayContext`, its request table and refusals), FR-075
  (`items_from_requirements`), FR-100 (`run`, `Call`, `RunRefusal`), FR-098
  (`replay` and its recompile), FR-255 (`CallerLimits`), FR-278 (the library
  operations), FR-060 (the T12-A boundary), ADR-011, ADR-013 O-04 (node keys
  carry no package), ADR-029.
