---
id: SR-17063
title: "IR-1007 PR #1470 integrity review"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language; spec/functional/FR-262-evaluate-values-and-calls-at-any-depth.md, spec/functional/FR-485-acquire-and-apply-a-checked-closure-through-the-replay-facade.md, spec/test-cases/TC-1263-checked-closure-acquisition-and-application-through-replay.md"
review_set: subset
---
# IR-1007 PR #1470 integrity review

## Summary

Ticket IR-1007; quire-spec-language PR #1470 frozen at <sha removed>. integrity reviewed FR-485 and TC-1263 against linked requirements and the public QSL/leaf contracts. An independent Claude Opus semantic audit was consulted; all findings were checked against the source.

## Verdict

**CONDITIONAL** — 2 finding(s) recorded.

## Scope

- FR-485: The `qsl-replay` facade SHALL expose checked-closure acquisition and application without giving a consumer access to `qsl_replay::spine`, the checker, or a second evaluator.
- FR-485-AC-1: For a checked unit with an own function and an imported dependency, acquisition returns package bytes, `package_id` and admitted tables equal to `compile_admitted` on the same inputs; an instrumented compile records one S1-to-E4 spine run. The handle remains usable after input buffers and the acquisition result's public package/context values are dropped.
- FR-485-AC-2: Lookup of an own callable's `WireNodeId` returns the checker-minted key and exact admitted signature; a nonexistent id refuses `Unknown`, an imported callable id refuses `Foreign` with that dependency's `package_id` in unit-scoped lookup, and an enum or parameter id refuses `WrongKind`. No consumer constructs or decodes a `NodeKey`.
- FR-485-AC-3: A CG-shaped external crate can acquire, look up, register a generated body and apply it using `qsl_replay` plus shared leaf/kernel/value crates, without any `qsl_replay::spine`, `qsl_eval`, `qsl_semantics` or `qsl_package` import; the same crate cannot obtain a QSL checked package or adapter through the facade.
- FR-485-AC-4: In one registry, a valid binding returns `Accepted`; repeating it returns `DuplicateBinding`; an unequal signature returns `SignatureMismatch`; an unknown key returns `CallableNotAdmitted`. A failed Begin leaves registration open, then a successful Begin seals before invocation/charge; later registration returns `RegistrySealed`, and two later sessions retain the first binding after an earlier session ends or drops.
- FR-485-AC-5: Applying an admitted interpreted `f(3)` returns the same `Evaluation`, losses, locations and meter spend as the current QSL call path over the same checked package; a registered generated body for `f` returns its generated result with one `function.call` charge. A generated body requesting another admitted callable receives its result through one leaf session, with no facade evaluator dispatch.
- FR-485-AC-6: An arity or type refusal at Begin makes no charge and invokes no body. With `work_units` set to one below the measured successful spend, application returns the leaf's located incomplete outcome at the denied charge, records only preceding charges, and completes when the limit is raised. Cancelling the meter's installed handle before its next charge produces the leaf's located incomplete outcome and the same `tripped()` cause; dropping a runnable session invokes no later body.
- FR-485-AC-7: A malformed source, same-owner dependency, S1 byte limit below source size, and S1 limit above the reader limit produce the same `AdmittedRefusal` class and stage/catalog detail as `compile_admitted`, with no handle or registry. A missing environment population is reported through the existing leaf/QSL refusal or incomplete vocabulary and is never fabricated as an admitted empty population.
- FR-485-AC-8: Source inspection and the T12-A CG checkout scan show the facade delegates to the one designated QSL application path (the existing std reference adapter before FR-405's switch), one QSL compile path, no new evaluator copy, no QSL re-export of leaf result/registry/session types, no CG import from a QSL module other than `qsl_replay`, and no compatibility shim or copied leaf source.
- TC-1263: Verify FR-485-AC-1 through FR-485-AC-8 through a consumer that may enter QSL only through `qsl_replay`, comparing the facade with the existing compile and evaluation paths.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The criterion mandates `Foreign` for an imported callable only "in unit-scoped lookup", but the Inputs/Behavior define no scope argument, alternate lookup, or rule for an imported callable outside unit scope. The retained closure and leaf registry include imported callables, so two facades could respectively reject or expose their checker-minted keys while both reading this text as compliant. Define the one lookup scope and whether imported callable registration is supported. | spec/functional/FR-485-acquire-and-apply-a-checked-closure-through-the-replay-facade.md:57 |
| FND-002 | medium | The missing-population fixture and oracle are not determined. `ObjectEnvironment::new` records no population; a missing `Population<T>` argument can fail at Begin, while a named-population read in an admitted body can reach a different incomplete/refusal path. AC-7 accepts any leaf/QSL refusal or incomplete result, and TC-1263 does not state which kind of missing read it exercises. Pin one adverse input and its expected arm and stage before and after FR-405, so a wrong path can fail the test. | spec/functional/FR-485-acquire-and-apply-a-checked-closure-through-the-replay-facade.md:62 |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-003 | high | The new unconditional assertion that a key obtained from a package outside the retained closure returns `CallableNotAdmitted` is unsound. `quire_eval::Registry::register` asks the closure for a signature by `NodeKey` equality alone; `PackageClosure::declaring` searches the retained graph for that identity and has no key-origin field. A second acquisition, or a different package retaining the same checked callable identity, can supply an equal key that is admitted here. State the intended key-equality behavior, or specify an enforceable provenance-bearing key boundary if origin must affect admission. | spec/functional/FR-485-acquire-and-apply-a-checked-closure-through-the-replay-facade.md:57; qsl-eval/src/value/expression/closure.rs:98-111; quire-eval/src/registry.rs:94-111 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | The new owner_package_id lookup returns own and imported callable keys and specifies Foreign, Unknown and WrongKind boundaries. |
| FND-002 | fixed | AC-7 and TC-1263 now pin the missing PopulationId fixture to argument admission and require WrongValueKind at parameter 0 on both evaluators. |
| FND-003 | still-open | The leaf's NodeKey equality does not carry origin, so the outside-package assertion is not enforceable for equal admitted keys. |

## New findings (disposition pass 2)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-004 | high | Behavior 5 still requires Begin to return `CallFailure::Input(InputRefusal::WrongValueKind)` directly for an unrecorded PopulationId, but this fix changes Begin to return `ApplicationBeginRefusal::Call(CallFailure)` and AC-7/TC-1263 explicitly require the wrapped `ApplicationBeginRefusal::Call(CallFailure::Input(...))`. These are different Rust result types and both obligations cannot hold. Update Behavior 5 to require the wrapped type as well. | spec/functional/FR-485-acquire-and-apply-a-checked-closure-through-the-replay-facade.md:47,62 |

## Dispositions (round 2)

| FND | outcome | reason |
| --- | --- | --- |
| FND-003 | fixed | AC-2 now returns an opaque acquisition-bound selection; registration and Begin reject selections from another acquisition before leaf delegation, including equal-key acquisitions. |
| FND-004 | still-open | Behavior 5 specifies the unwrapped CallFailure return despite the new typed ApplicationBeginRefusal contract. |

## Dispositions (round 3)

| FND | outcome | reason |
| --- | --- | --- |
| FND-004 | fixed | Behavior 5 now returns ApplicationBeginRefusal::Call(CallFailure::Input(...)), matching Outputs, Behavior 6 and 8, AC-7 and TC-1263. |

## Incremental review: root-site amendment at <sha removed>

The frozen amendment changed FR-262, FR-485 and TC-1263 after the original review and three dispositions. This same reviewer examined FR-262-AC-13, FR-485 Behavior 3/4/6 and AC-6, and TC-1263 steps/expected results 5 against QSL `PackageClosure`, `CheckedGraph`, `Location`, and quire-eval `Registry`/`Session`. The earlier FND outcomes remain fixed. Quire validation of the three changed spec files passed. No acceptance criterion was narrowed; the root-site failure is typed and leaves Begin open; pre-Begin cancellation remains distinct from post-Begin denial; the leaf change precedes the facade; one evaluator path and T12-A remain specified.

- FR-262-AC-13: Begin of an own callable and of an imported dependency callable obtains each callable's checked body root `Location` from its declaring checked graph. After successful Begin, cancelling the meter's installed handle before the first `function.call` charge returns a terminal incomplete evaluation whose `location` is exactly that graph's checked body root location, with no admitted charge and no body invocation; a work-limit denial at that charge has the same location. An admitted callable whose closure cannot return that location makes Begin return `CallFailure::Fault` with stage `quire-eval`, invariant `root-call-site-absent`, no session or charge, and an open registry. Begin has no caller-supplied location parameter; nested child requests continue to use their own `site`.
- FR-485-AC-6: An arity or type refusal at Begin returns `ApplicationBeginRefusal::Call` carrying the leaf refusal, with no charge or body invocation. Cancellation of the meter's installed handle with `Requested` before Begin returns `ApplicationBeginRefusal::Call(CallFailure::Cancelled(Requested))` with no session or charge. After a successful Begin of an own callable and, separately, an imported callable, cancellation with `Requested` before the session's first `function.call` charge returns the leaf's incomplete evaluation at that charge with `location` exactly equal to the declaring checked graph's function-body root `Location`, no admitted charge and `tripped() = Requested`; neither result has `location: None` or the other package's body location. A `work_units` denial at root `function.call` has the same location. With `work_units` one below the measured successful spend, application returns the leaf's located incomplete outcome at the denied charge, records only preceding charges, and completes when the limit is raised. Dropping a runnable session invokes no later body.
- TC-1263 step 5: 5. Apply `f(3)` without and with a generated body; have the generated body request another admitted callable. Compare the interpreted run with QSL's current result, location, loss and spend. Repeat at `work_units` one below and exactly equal to measured spend, and with wrong arity and wrong type. Cancel the meter's installed handle with `Requested` before `begin_call` in one run. In two other runs, begin successfully with an uncancelled handle, once for the unit's `f` and once for an imported dependency callable; cancel the handle before each session's first `function.call` charge, then step. Repeat the root charge denial with a `work_units` limit of zero. Compare each terminal `Evaluation.location` with the actual `function_by_identity(callable).body.location()` in the callable's declaring checked graph, using a QSL-internal oracle; also compare the own and imported locations to ensure the selected unit's location was not substituted for the imported one. In a leaf contract test, admit a callable whose checked closure returns no root location, then call Begin.

No new finding in this analysis lens at the frozen amendment head.

## Disposition round 5

The two-file fix at <sha removed> introduces no new finding in this analysis lens.
