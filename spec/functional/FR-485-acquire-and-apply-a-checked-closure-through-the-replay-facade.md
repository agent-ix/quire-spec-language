---
id: FR-485
title: "Acquire and apply a checked closure through the replay facade"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-060
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-262
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-401
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-405
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-394
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: traces_to
---
# FR-485: Acquire and apply a checked closure through the replay facade

## Description

A CG consumer permitted by [FR-060](FR-060-check-qsl-api-surface-boundary.md) T12-A can obtain checked-package bytes through `qsl_replay::compile_package`, but those bytes are not an in-process checked closure. The `quire-eval` leaf has the [FR-262](FR-262-evaluate-values-and-calls-at-any-depth.md) registry and session interface; [FR-401](FR-401-host-the-evaluation-interface-in-the-quire-eval-leaf.md) places the QSL adapter to that interface in `qsl-eval`. The `qsl-replay` facade SHALL expose checked-closure acquisition and application without giving a consumer access to `qsl_replay::spine`, the checker, or a second evaluator. This is an in-process API; it adds no wire format or checked authority.

## Inputs

- `acquire_checked_closure(source, path, bytes, packages, dependencies, limits, replay_limits)`: exactly the seven inputs and limit meanings of `compile_admitted` and `compile_package`.
- A `CheckedClosureCompilation` returned by acquisition; an admitted callable `NodeKey` obtained from that compilation's checked package by the facade lookup; optional host `BodyFactory` bindings with their leaf `Signature` values; an application environment; completed `Vec<quire_exact::Value>` arguments; and a caller-owned `quire_exact::Meter` with its optional installed cancellation handle.

## Outputs

- A `CheckedClosureCompilation` with the identical emitted `CompiledPackage` and `AdmittedContext` of `compile_admitted` for the same inputs, plus an opaque `CheckedClosureHandle` that retains the checked package and its exact direct and transitive dependency closure.
- A facade-owned `ApplicationRegistry` over that handle; registration returns `quire_eval::Admission`. A successful Begin returns an opaque `CheckedApplicationSession`; stepping returns runnable or terminal, and running returns `Result<quire_eval::Evaluation, quire_eval::CallFailure>`. The caller's meter records the leaf's consumed counters and charges.
- Acquisition refusals use `AdmittedRefusal`; a facade callable lookup uses `Unknown`, `Foreign(package_id)` or `WrongKind` as applicable; application uses the leaf's existing `CallFailure` and `Evaluation` vocabulary.

## Behavior

### Acquisition and identity

1. `acquire_checked_closure` SHALL call the same `recompile_unit` path as `compile_package` and `compile_admitted` exactly once, under the same S1-to-E4 limits and dependency-owner check. It SHALL derive the emitted package, admitted context and retained checked closure from that one result. It SHALL NOT parse emitted bytes back into a second checked graph, construct a `NodeKey`, or run a second checker or evaluator.
2. The handle SHALL own or share the checked package and its dependency closure, so dropping the acquisition result's emitted bytes, admitted context, original source buffers or dependency input after acquisition does not invalidate a registry or live session. Its package identity SHALL equal the emitted package's `package_id`. A handle from a different acquisition SHALL NOT be substituted into an existing registry or session.
3. The facade SHALL resolve a `WireNodeId` from the emitted package or a `FunctionSite` against this exact retained closure. A node admitted as a callable returns the check-minted `NodeKey` and its admitted `quire_eval::Signature`; a missing node refuses `Unknown`, a node owned by an imported package refuses `Foreign` naming that package's id when the lookup is scoped to this unit, and a non-callable node refuses `WrongKind`. The facade SHALL NOT decode, hash, or synthesize an admitted key in the consumer.

### Application

4. `CheckedClosureHandle::registry()` SHALL create a fresh facade-owned `ApplicationRegistry` from the retained closure through the designated QSL `CheckedClosure` adapter and `quire_eval::Registry`. Before [FR-405](FR-405-retire-the-std-reference-evaluator-for-quire-eval.md)'s switch, that adapter is `qsl_eval::value::expression::PackageClosure` over the std reference evaluator. At the switch, this same facade SHALL delegate to the replacement `quire-eval` evaluator; it SHALL offer no selector, fallback, dual run or retained adapter. A registry SHALL use the same checked closure for all registration, Begin and subsequent sessions. The facade SHALL not expose the adapter, checked package or `qsl_replay::spine` as a consumer API, and SHALL not re-export a moved `quire_eval` or `quire_semantic_value` type from QSL.
5. The facade SHALL accept a `quire_semantic_value::object_closure::ObjectClosure` and form an opaque `ApplicationEnvironment` through QSL's existing `ObjectEnvironment::new`; it SHALL invent no population, object, snapshot or read. If a function needs information absent from that closure, the leaf's existing input, storage, undefined or incomplete result SHALL be returned as applicable. A later admitted-snapshot producer may supply an environment through a separate requirement; this API SHALL NOT silently treat missing model data as empty admitted data.
6. `ApplicationRegistry::register(callable, signature, factory)` SHALL delegate to the leaf registry, including its ordered `RegistrySealed`, `CallableNotAdmitted`, `SignatureMismatch`, `DuplicateBinding`, `Accepted` results. `begin_call(callable, arguments, environment, meter)` SHALL delegate admission and sealing to leaf `begin_call`; it takes no separate cancellation handle or untrusted package. A failed Begin leaves an open registry open. A successful Begin seals it before any body invocation or charge; ending or dropping a session never reopens it. A new binding set needs a fresh registry.
7. `CheckedApplicationSession::step` and `run` SHALL delegate to the same `quire_eval::Session`, preserving the leaf's ordered charges, locations, losses, child requests, cancellation and terminal outcome. The facade SHALL not evaluate a checked expression or callable by its own node dispatch. A session is consumed at terminal; dropping a runnable session runs no continuation. The caller may retain and inspect its own meter after the session ends.
8. On acquisition failure, the facade SHALL return the same `AdmittedRefusal` class and stage/catalog detail as `compile_admitted` over the same input and limits and SHALL return no handle. A lookup refusal or registration refusal SHALL invoke no body and make no application charge. A root argument-admission failure SHALL return the leaf `CallFailure` with no application charge; a denied `function.call` or body charge SHALL retain preceding spend and return the leaf's located incomplete evaluation. Cancellation SHALL follow only the meter's installed handle and the leaf's poll and charge points.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-485-AC-1 | For a checked unit with an own function and an imported dependency, acquisition returns package bytes, `package_id` and admitted tables equal to `compile_admitted` on the same inputs; an instrumented compile records one S1-to-E4 spine run. The handle remains usable after input buffers and the acquisition result's public package/context values are dropped. | Test |
| FR-485-AC-2 | Lookup of an own callable's `WireNodeId` returns the checker-minted key and exact admitted signature; a nonexistent id refuses `Unknown`, an imported callable id refuses `Foreign` with that dependency's `package_id` in unit-scoped lookup, and an enum or parameter id refuses `WrongKind`. No consumer constructs or decodes a `NodeKey`. | Test |
| FR-485-AC-3 | A CG-shaped external crate can acquire, look up, register a generated body and apply it using `qsl_replay` plus shared leaf/kernel/value crates, without any `qsl_replay::spine`, `qsl_eval`, `qsl_semantics` or `qsl_package` import; the same crate cannot obtain a QSL checked package or adapter through the facade. | Test |
| FR-485-AC-4 | In one registry, a valid binding returns `Accepted`; repeating it returns `DuplicateBinding`; an unequal signature returns `SignatureMismatch`; an unknown key returns `CallableNotAdmitted`. A failed Begin leaves registration open, then a successful Begin seals before invocation/charge; later registration returns `RegistrySealed`, and two later sessions retain the first binding after an earlier session ends or drops. | Test |
| FR-485-AC-5 | Applying an admitted interpreted `f(3)` returns the same `Evaluation`, losses, locations and meter spend as the current QSL call path over the same checked package; a registered generated body for `f` returns its generated result with one `function.call` charge. A generated body requesting another admitted callable receives its result through one leaf session, with no facade evaluator dispatch. | Test |
| FR-485-AC-6 | An arity or type refusal at Begin makes no charge and invokes no body. With `work_units` set to one below the measured successful spend, application returns the leaf's located incomplete outcome at the denied charge, records only preceding charges, and completes when the limit is raised. Cancelling the meter's installed handle before its next charge produces the leaf's located incomplete outcome and the same `tripped()` cause; dropping a runnable session invokes no later body. | Test |
| FR-485-AC-7 | A malformed source, same-owner dependency, S1 byte limit below source size, and S1 limit above the reader limit produce the same `AdmittedRefusal` class and stage/catalog detail as `compile_admitted`, with no handle or registry. A missing environment population is reported through the existing leaf/QSL refusal or incomplete vocabulary and is never fabricated as an admitted empty population. | Test |
| FR-485-AC-8 | Source inspection and the T12-A CG checkout scan show the facade delegates to the one designated QSL application path (the existing std reference adapter before FR-405's switch), one QSL compile path, no new evaluator copy, no QSL re-export of leaf result/registry/session types, no CG import from a QSL module other than `qsl_replay`, and no compatibility shim or copied leaf source. | Inspection |

## Dependencies

- [FR-060](FR-060-check-qsl-api-surface-boundary.md) T12-A and shared compile path; [FR-394](FR-394-expose-the-admitted-enum-key-table-through-the-replay-facade.md) context; [FR-262](FR-262-evaluate-values-and-calls-at-any-depth.md) and [FR-401](FR-401-host-the-evaluation-interface-in-the-quire-eval-leaf.md) shared leaf interface; [FR-405](FR-405-retire-the-std-reference-evaluator-for-quire-eval.md) later changes the designated evaluator behind this same facade.
