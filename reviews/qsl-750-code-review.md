---
id: SR-12141
title: "Code review of quire-spec-language PR #964: model constructs gated on the model profile selection"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@ae6dacabdb6bab1f89a670238fa60dc98b41c53b; PR #964: qsl-semantics/src/check/{model_gate,layer,assemble,mod}.rs, qsl-semantics/src/value/{definition,layer,mod}.rs, qsl-package/src/emit.rs, qsl-replay/src/spine.rs, qsl-replay/src/spine/refinement.rs, qsl-replay/src/spine/clause/tests/model_gate.rs, qsl-replay/src/spine/call/tests.rs, qsl-replay/tests/dispatch_link_compile.rs, qsl-semantics/tests/it/{complete_value_lock,model_operations}.rs, xtask/src/conformance.rs, xtask/tests/fixtures/checks.rs, spec/functional/{FR-102,FR-302,FR-361}"
review_set: subset
---
# Code review of quire-spec-language PR #964

## Summary

Ticket: QSL-750. Code review with the rust-review lane
(agent-skills/rust-review/SKILL.md) folded in. Read-only, static review of
`git diff origin/main...HEAD` (base bec89ae44). No cargo or make run (disk
constraint). The PR body reports `cargo check` and `cargo clippy` only. The
full `make ci` receipt is still pending.

What was checked by hand:

- Gate soundness. `model_gate::refusals` runs in `assemble_with_cancel`.
  Every checked-package path goes through it: spine `compose`/`run_spine`,
  `compile_package`, replay recompile, S4 library compile, and every
  `PackageDeclarations::assemble` caller. The gate reads only the header
  profile identities (`HeaderLayer::from_identity == ModelComplete`). It does
  not infer the profile from a `model` declaration, an alias or a construct,
  and it never falls back to `value.complete`. A body refuses
  `declaration-form` at the declaration span. A dispatched call or a lookup
  refuses `expression-form` at the node span. It walks function bodies,
  `decreases` measures, state-clause bodies and operation-body bodies,
  reports every construct, and sorts by span. All correct.
- No double refusal. `LayerAdmission::refuses` skips a model-construct form
  when the unit does not select the model profile (FR-361 item 12). In a
  selecting unit, FR-110's layer rule refuses alone, because the model row now
  admits `OperationBody`, `DispatchCall` and `Lookup` and no lower row does.
  `operation_bodies` now runs the layer rule as well. Correct.
- E3 precedence: a profile refusal returns before assembly (AC-9). Correct.
- Lock: `ModelRole::ALL` has 18 rows with role, authority and identity. I
  compared them with QSpec's complete-model lock by id: same rows, same
  order, `package_selection.always` is all 18, and `conditional` and
  `exactly_one` are empty. Nothing pins `artifact_path`, a digest or a
  version. The emitter's dedup absorbs the `root`/`state_graph` rows that
  repeat the layer rows. This is a canonical lock selection, not file
  tracking. The xtask entry uses `SummaryMode::Exact`, so a skipped run
  (with `QSPEC_DIR` unset) cannot pass `make conformance`.
- The value.complete `ObjectNavigation` exception (STD-161) is untouched.

Rust idioms: exhaustive matches with no wildcard on `LayerForm`,
`ModelFamily` and `AssemblyCause`. No `unwrap` in library code, no `unsafe`,
and no compatibility layer.

## Verdict

Request changes: one high (FND-001). FND-002 is a medium, recorded. FND-003
and FND-004 are lows, recorded only.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The fixture migration is incomplete. The qsl-eval suites still assemble value-profile units that hold `lookup` and dispatched calls, and they `.expect` the assembly to succeed. The new gate refuses those units at S3, so these tests panic and `make ci` goes red. In `state_model_family.rs`, `declarations()` uses `profile v = "quire.value.complete/v1"`, and its callers include `lookup<M::A>(p,r)` (lines 284, 327, 426, 540, 787, 797) and `self.ready()`/`self.missing()` (lines 299, 329, 433, 607) under `.expect("assembly succeeds before any S3 assertion")`. In `model_simulation.rs`, `build_world` uses a value profile with `pre Low ... { self.ready() }` (lines 1519, 1540, 1550) under `.expect("the unit assembles")`. Migrate these headers to `quire.model.complete/v1` and sweep the other model-backed fixtures, then run the gate. | qsl-eval/tests/it/state_model_family.rs:117; qsl-eval/tests/it/state_model_family.rs:284; qsl-eval/tests/it/state_model_family.rs:299; qsl-eval/tests/it/model_simulation.rs:254; qsl-eval/tests/it/model_simulation.rs:1519 |
| FND-002 | medium | The gate does not walk the `ExpressionForm` sites: protocol control guards, invariants and variants, `visible`, the `fifo` key, `terminal ... when`, and temporal `holds`/`when`/capture values. FR-361 item 1 ("each such construct the unit itself writes") and item 13 require them. Item 8 exempts protocol and temporal forms, not model constructs written inside them. Today each of those sites refuses for another reason, so no package escapes: `protocol_clause::content` refuses `not-yet-implemented`, `terminal` refuses `UnsupportedDeclarationForm`, and temporal refuses `not_emitted`. But a dispatch or lookup there gets the wrong cause and no `dispatch-call`/`lookup` family. The gate also becomes bypassable as soon as one of those checkers lands. | qsl-semantics/src/check/model_gate.rs:131-205; qsl-semantics/src/check/assemble.rs:2442-2450; qsl-semantics/src/check/protocol_clause/coverage.rs:61-92 |
| FND-003 | low | Errors are sorted by `(start, end)`. A chained `a.f().g()` puts the inner call before the outer one, and both start at the same byte, while the walk's pre-order would put the outer first. Both readings are defensible "source order", but the order is not stated. | qsl-semantics/src/check/model_gate.rs:204 |
| FND-004 | low | A function whose `spans()` is `None` is skipped by the gate (`continue`). This cannot happen from S2, because `ParsedUnit` is built only from source. The skip should still refuse at the `using` span, as the layer rule does, rather than skip silently. | qsl-semantics/src/check/model_gate.rs:153-155 |

## Dispositions

Round 1, reviewed at `3728ed91f0da4d8a0f53b600b81aeec0f2e2cade`. The fix commits are `dd535b3bd` and `3728ed91f`; the merges of main are excluded. This round was static: no cargo or make run.

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | dd535b3bd: `state_model_family.rs` `declarations()` and `model_simulation.rs` `build_world` now select `quire.model.complete/v1`. 3728ed91f moves the remaining model-backed fixtures, including `model_conformance_boundary.rs`, the navigating fixtures and `spine-model.native`. A grep sweep found no other unit that holds a dispatched call, `lookup` or `body` under a value-only header and builds past S2. The lead's `make ci` receipt is still the gate. |
| FND-002 | accepted-no-change | Medium, recorded under the review cap. Every unwalked `ExpressionForm` site still refuses another way, so no package escapes. This must be revisited when the protocol, terminal or temporal expression checkers land. |
| FND-003 | accepted-no-change | Low, recorded only. Both orders are defensible source orders. |
| FND-004 | accepted-no-change | Low, recorded only. The skip cannot be reached from S2. |

Also checked this round: FR-361 item 17 and AC-13 (STD-161).

- (a) The `ObjectNavigation` entry and its exception comment are gone from the `ValueComplete` row. `NAVIGATION` in `layers.rs` is now `[StateGraph, ModelComplete]`.
- (b) The FR-110 exception bullet is deleted.
- (c) Every fixture that writes `deref`/`reaches` under a value alias moves to the model or graph profile. The remaining `deref`/`reaches` texts are S1/S2-only tests or legacy parse fixtures.
- `object_navigation_is_not_a_value_layer_form` checks that `deref` and `reaches` each get one `UnsupportedLayerForm { ValueComplete, ObjectNavigation }`, under a value alias alone and with a second graph alias, and compile under graph and model. That matches AC-13 and TC-1015 step 3.

No new findings.
