---
id: SR-2026111015000000000007
title: "code-review (with rust-review lane) of QSL PR 2675 checked dispatch bridge TypeEnvironment"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; PR #2675 @ 1724d1035; qsl-semantics/src/check/checked_dispatch.rs, qsl-semantics/src/check/dispatch_link.rs, qsl-semantics/src/model/intake.rs, qsl-eval/tests/it/dispatch_calls.rs, spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md"
review_set: subset
---
# SR-2026111015000000000007: code-review of QSL PR 2675

## Summary

Ticket: AGE-1959. PR #2675 head 1724d1035. `checked_dispatch_operation` now returns a
`PackageDeclarations::types` built by a new private `object_type_environment` from the view's
`ObjectType` records and `object_type_supertypes`. Admission failures become
`DispatchBridgeRefusal::TypeEnvironment(Box<EnvironmentFailure>)`, which `link_causes` maps to
`runtime_invariant`. The Rust lane (rust-review, rust-style) ran in this same artifact.

Gates run on the PR worktree: `cargo fmt --check` clean; `cargo clippy -p qsl-semantics -p qsl-eval
--all-targets --all-features -D warnings` clean; `cargo test -p qsl-eval --all-features --test it
dispatch_calls::` 38 passed; `cargo test -p qsl-semantics --all-features dispatch` 17 + 16 passed.

Probes ran on a scratch clone with its own target dir:

- An `ObjectType` `model.A` whose supertype is a `Process` `model.P` (with identity field `P/id`) and a
  query `model.A/size`. Normalization completes under default limits. On the PR head,
  `checked_dispatch_operation` returns `Err(TypeEnvironment(Refused(InvalidDeclaration {
  declaration: "model.A", cause: UnknownObjectType(model.P) })))`. On the parent commit
  (1724d1035~1) the same input returns `Ok`.
- The new test against parent bridge code fails at `declarations.check()` with `IllTyped(TypeMismatch)`
  in both bodies.
- The PR head with `.with_supertypes` dropped fails the new test at `check_clause_expression`'s
  `expect` (dispatch_calls.rs:2174). The oracle asserts a check result and pins the supertype edge.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | `object_type_environment` declares only `DomainPackageRecord::ObjectType`. Normalization flags a `Process` as an object type (model/index.rs:720-721), so an object type may specialize a process, and assembly declares `Process`, `StateMachine` and `PersistenceInterface` as object types (assemble.rs:1673-1714). For an object type under a process supertype, `TypeEnvironment::new` refuses `UnknownObjectType`. The bridge returns `TypeEnvironment`, `link_causes` maps it to `runtime_invariant`, and `link_dispatch` parks the family as pending. A valid model plus a valid `a.size()` call that linked before this PR now refuses as a broken invariant. Reproduced: Err on PR head, Ok on parent. | qsl-semantics/src/check/checked_dispatch.rs:373-396, qsl-semantics/src/check/checked_dispatch.rs:1253, qsl-semantics/src/check/dispatch_link.rs:639 |
| FND-002 | medium | Production `link_dispatch` throws the new environment away: `merge_linked` destructures `PackageDeclarations { .., }` and never reads `types` (dispatch_link.rs:362-369). Each called family now builds a whole-view TypeEnvironment, including its ancestor closure, and drops it. That adds a failure path (FND-001, FND-003) and no production value. It is also a second builder of model object types beside assemble.rs:1673-1714, with a different name format (`key.node` vs `alias::artifact`), different record kinds, no attributes and different limits. Either share one builder or keep the build out of the production link path. | qsl-semantics/src/check/checked_dispatch.rs:373-396, qsl-semantics/src/check/dispatch_link.rs:362-369, qsl-semantics/src/check/assemble.rs:1673-1714 |
| FND-003 | medium | Every `EnvironmentFailure` arm maps to `runtime_invariant`. Refused `UnknownDeclaration`, `GeneralizationCycle` and `DuplicateKey` are unreachable from a normalized view: normalize refuses `UnknownGeneral` (normalize.rs:1963-1977) and `specialization-cycle`, and the re-key refuses `DuplicateObjectKey` first. `Limit`, `Allocation` and `Capacity` are reachable, though. The bridge uses `TypeEnvironment::new` (default 16,777,216 work units), while assembly admits under the caller's `AssemblyLimits.environment` (FR-255, assemble.rs:4655-4661). Scenario: a caller raises `environment.work_units`, and a ~5,800-type chain (closure > 16,777,216 units) admits in assembly. The bridge then refuses `Limit(WorkUnits)`, reported as `runtime_invariant` rather than `stage_limit_exceeded`. Allocation failure likewise reports as invariant rather than `resource_exhausted` (contrast `environment_cause`, assemble.rs:3169-3184). Traced, not run. | qsl-semantics/src/check/checked_dispatch.rs:394-395, qsl-semantics/src/check/dispatch_link.rs:636-639 |
| FND-004 | low | The doc comment says "The bridge's clauses name no attributes, so none are declared". That is false: `OperationClauses` carries caller bodies and preconditions, and `link_dispatch` supplies the unit's own `pre` clauses and `operation-body` bodies, which can read `self.attr`. A direct caller that `check()`s the returned declarations with such a clause gets a missing-attribute refusal. Production is unaffected only because it discards `types` (FND-002). State what the environment omits, or populate attributes from the shared builder. | qsl-semantics/src/check/checked_dispatch.rs:369-372 |
| FND-005 | low | Hand-built environments still overwrite the bridge's. The D01 test rebuilds the identical environment from `object_type_supertypes`, asserts `conforms` on its own copy, and assigns it over `declarations_a.types`. `ab_bridge_declarations`, the rename test and the inherited-static-type test replace it with weaker environments that have no supertypes. Those tests no longer exercise the bridge's environment, and the D01 asserts test a copy. Delete the overrides and assert on `declarations.types`. | qsl-eval/tests/it/dispatch_calls.rs:2262-2284, qsl-eval/tests/it/dispatch_calls.rs:1193-1201, qsl-eval/tests/it/dispatch_calls.rs:1812-1819, qsl-eval/tests/it/dispatch_calls.rs:2606-2615 |
| FND-006 | low | The ADR-013 S-1 row is half-updated. It now contrasts the kernel `Value::Population(PopulationId)` with "the QSL copy [that] carries a binding", but no QSL `Value` copy exists at HEAD (only `quire_exact::Value`). The next sentence still "directs #213 S-1b to add" the variant, which now exists. | spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md:1254 |

## Verdict

Not mergeable: FND-001 is a reproduced regression on a valid model. The boxing of
`EnvironmentFailure` is consistent with the enum's other boxed variants. The `filter_map` `?` arms in
`object_type_environment` cover unreachable cases (the re-key already refused them); that is a style
note, not a finding. The new test's oracle is real: it asserts a check result, and dropping the
supertype edge fails it. Its red-on-old-code reason is the empty environment (`TypeMismatch` at
`check()`), not the subtype edge alone. The intake.rs doc fixes match the record types that exist.

## Dispositions

Renumbered in disposition pass 1 from SR-2026111015000000000002 to SR-2026111015000000000007: SR-2026111015000000000002 collided with QSL-1400's spec reviews (#2674, on main). The review-pass Linear comment carries SR-2026111015000000000002.

Disposition pass 1, against fix commit 662c24b80. The lead's design is applied: the bridge takes `types` from its caller,
`link_dispatch` passes the unit's assembled environment, and test callers use the test-support
`model_type_environment` (assemble.rs), which runs the same `model_object_types`/`admit_types` as assembly.
`object_type_environment` and `DispatchBridgeRefusal::TypeEnvironment` are deleted.

Focused gates on the worktree target:
- qsl-eval `dispatch_calls::` + `state_model_family::`: 51 passed.
- qsl-semantics lib `dispatch` + `lowering::model`: 57 passed.
- qsl-semantics `it` `model_dispatch::`: 13 passed, **2 failed** (FND-009).
- qsl-package `owners`: 5 passed.
- clippy (`-D warnings`, all targets and features) and fmt: clean.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | The bridge carries the caller's environment; `link_dispatch` passes the assembled one, which declares Process as an object type. `bridge_links_an_object_type_specializing_a_process` checks the family over an environment admitted by assembly's own builder. |
| FND-002 | fixed | The bridge builds no environment and has no second builder; `model_type_environment` reuses `model_object_types`/`admit_types`. The remaining per-family clone is FND-008. |
| FND-003 | fixed | The bridge's admission refusal is deleted; environment failures now arise only in assembly, mapped by `environment_cause`. |
| FND-004 | fixed | `object_type_environment` and its doc are deleted; the bridge doc says `types` is the unit's assembled environment. |
| FND-005 | fixed | D01 reads `declarations_a.types`; the hand-built overrides at the four cited sites are removed. One leftover, at a site not cited, is FND-007. |
| FND-006 | fixed | The S-1 row drops the QSL-copy clause and the "directs S-1b to add" sentence. T-6, O-13 and QC-21 now state `PopulationId` as the population node's `NodeId`, matching QSpec FR-201:64 and quire-exact `PopulationId(NodeKey)`. |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-007 | low | The money-bridge fixture passes `model_type_environment(..)` to the bridge, then overwrites `declarations.types` with a hand-built environment that adds the unit composites. The environment it passes is built and discarded. Pass the hand-built `types` as the argument instead. | qsl-eval/tests/it/dispatch_calls.rs:3037-3057 |
| FND-008 | medium | `link_dispatch` passes `self.types.clone()` into each called family's bridge call. `merge_linked` then drops it (`..`, dispatch_link.rs:362-369), and the bridge never reads `types` itself. `TypeEnvironment` has no shared storage: the clone deep-copies composites, object types, the ancestor closure (up to O(N²)) and every effective attribute set. Cost is O(families × environment size), charged to no meter. `mem::take` around the loop does not fit: each call consumes the value, and a refusal does not return it. A borrow alone cannot fill an owned field. Right shape: a crate-private bridge core that takes no environment, used by `link_dispatch`, and a public wrapper that takes an owned `TypeEnvironment` and sets `types` for standalone callers. That also removes the `too_many_arguments` allow from the production function. | qsl-semantics/src/check/dispatch_link.rs:173-175, qsl-semantics/src/check/checked_dispatch.rs:905-914 |
| FND-009 | high | Two qsl-semantics integration tests fail on the PR head. `a_dispatch_family_with_more_than_128_redefinition_steps_passes_the_checked_bridge` (model_dispatch.rs:691) and `the_checked_bridge_counts_every_walk_against_one_family_steps` (model_dispatch.rs:768) both panic at `model_type_environment(..).unwrap()` with `AssemblyRefusal { ModelType { alias: "fixture", node: "model.A" } }`. Their `chain_package` fixture still uses `model.A` keys, which assembly's builder refuses; the fixture keys were rewritten in dispatch_calls.rs but not here. These are the TC-225 / FR-083-AC-4 tests. Reproduced twice on the worktree target, with a deterministic assertion message. | qsl-semantics/tests/it/model_dispatch.rs:691, qsl-semantics/tests/it/model_dispatch.rs:768 |
