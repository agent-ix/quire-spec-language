---
id: SR-17027
title: "IR-1007 PR #1483 gap-analysis of checked-closure facade"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; IR-1007; PR #1483 at 87a987234 vs e66db517; qsl-replay/src/application.rs, qsl-eval/src/value/expression/closure.rs, qsl-replay/src/admitted.rs, execute.rs, lib.rs, spine/lifecycle.rs, Cargo.toml, Cargo.lock, tests/fixtures/checked-closure-consumer"
review_set: subset
---

## Summary

Plan completion: not assessed. Checked the FR-485 acceptance criteria against the tagged TC-1263 test functions and production path in the frozen PR diff. Full-root quoin matrix stalled on the shared host and was stopped; this is a manual scoped trace and evidence check, not a repository-wide matrix claim.

## Verdict

**CONDITIONAL.** AC-6 has two material test-evidence gaps; the implementation path was statically consistent, but its location and runnable-drop obligations are not proven.

## Reviewed scope

- `FR-485-AC-1` — examined: For a checked unit with an own function and an imported dependency, acquisition returns package bytes, `package_id` and admitted tables equal to `compile_admitted` on the same inputs; an instrumented compile records one S1-to-E4 spine run. The handle remains usable after input buffers and the acquisition result's public package/context values are dropped.
- `FR-485-AC-2` — examined: `lookup_callable(unit_package_id, own_callable_id)` returns an opaque selection carrying the checker-minted key and exact admitted signature. With the imported callable's id and the unit owner it refuses `Foreign` naming the dependency's `package_id`; with the dependency's `package_id` it returns a selection that registers and applies in the same closure. An absent id or owner refuses `Unknown`, and an enum or parameter id in its owning package refuses `WrongKind`. No consumer constructs or decodes a `NodeKey` as application authority.
- `FR-485-AC-3` — examined: A CG-shaped external crate can acquire, look up, register a generated body and apply it using `qsl_replay` plus shared leaf/kernel/value crates, without any `qsl_replay::spine`, `qsl_eval`, `qsl_semantics` or `qsl_package` import; the same crate cannot obtain a QSL checked package or adapter through the facade.
- `FR-485-AC-4` — examined: In one registry, a valid selection/binding returns `Accepted`; repeating it returns `DuplicateBinding`; an unequal signature returns `SignatureMismatch`. An internal malformed-selection probe whose key is absent from the retained closure returns the leaf's `CallableNotAdmitted`; the public lookup refuses that absent id as `Unknown` and issues no selection. A failed Begin leaves registration open, then a successful Begin seals before invocation/charge; later matching-selection registration returns `RegistrySealed`, and two later sessions retain the first binding after an earlier session ends or drops. A selection from a second acquisition, including one with the same `package_id`, `NodeKey` and signature, returns `SelectionRefusal::WrongAcquisition` at register and `ApplicationBeginRefusal::WrongAcquisition` at Begin, before leaf delegation and without changing the registry or meter, even if the registry is sealed.
- `FR-485-AC-5` — examined: Applying an admitted interpreted `f(3)` returns the same `Evaluation`, losses, locations and meter spend as the current QSL call path over the same checked package; a registered generated body for `f` returns its generated result with one `function.call` charge. A generated body requesting another admitted callable receives its result through one leaf session, with no facade evaluator dispatch.
- `FR-485-AC-6` — examined: An arity or type refusal at Begin returns `ApplicationBeginRefusal::Call` carrying the leaf refusal, with no charge or body invocation. Cancellation of the meter's installed handle with `Requested` before Begin returns `ApplicationBeginRefusal::Call(CallFailure::Cancelled(Requested))` with no session or charge. After a successful Begin of an own callable and, separately, an imported callable whose checked-body root `Location` differs from the own callable's, cancellation with `Requested` before the session's first `function.call` charge returns the leaf's incomplete evaluation at that charge with `location` exactly equal to the declaring checked graph's function-body root `Location`, no admitted charge and `tripped() = Requested`; neither result has `location: None` or the other package's body location. A `work_units` denial at root `function.call` has the same location. With `work_units` one below the measured successful spend, application returns the leaf's located incomplete outcome at the denied charge, records only preceding charges, and completes when the limit is raised. Dropping a runnable session invokes no later body.
- `FR-485-AC-7` — examined: A malformed source, same-owner dependency, S1 byte limit below source size, and S1 limit above the reader limit produce the same `AdmittedRefusal` class and stage/catalog detail as `compile_admitted`, with no handle or registry. Begin of `h(p: Population<T>[3]): Boolean` with a `Value::Population` whose id is not recorded in `ApplicationEnvironment` returns `ApplicationBeginRefusal::Call(CallFailure::Input(InputRefusal::WrongValueKind))` naming parameter `0`, with no session, charge or body read, before and after FR-405's switch; no admitted empty population is fabricated.
- `FR-485-AC-8` — examined: Source inspection and the T12-A CG checkout scan show the facade delegates to the one designated QSL application path (the existing std reference adapter before FR-405's switch), one QSL compile path, no new evaluator copy, no QSL re-export of leaf result/registry/session types, no CG import from a QSL module other than `qsl_replay`, and no compatibility shim or copied leaf source.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | AC-6 root-denial tests accept any Some(location) and exercise only the own callable; they never verify the exact checked-body root Location for own and imported callables, distinct imported site, or root work-denial location. | qsl-replay/src/application.rs:876 |
| FND-002 | medium | No test steps a session to Runnable, drops it, and proves that no later generated or interpreted body invocation occurs, despite the explicit AC-6 drop contract. | qsl-replay/src/application.rs:876 |

## Coverage

All eight FR-485 ACs were compared to production code and tagged tests. AC-6 remains insufficiently evidenced. No plan selected. Plan completion: not assessed. The full-root quoin matrix did not complete and was stopped after roughly two minutes of no output; static tags were inspected directly.
