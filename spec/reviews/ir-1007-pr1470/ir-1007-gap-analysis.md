---
id: SR-17025
title: "IR-1007 PR #1470 gap-analysis review"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; spec/functional/FR-485-acquire-and-apply-a-checked-closure-through-the-replay-facade.md, spec/test-cases/TC-1263-checked-closure-acquisition-and-application-through-replay.md"
review_set: subset
---
# IR-1007 PR #1470 gap-analysis review

## Summary

Ticket IR-1007; quire-spec-language PR #1470 frozen at 20fd130e5b66c37745bbc16a26f21089033bfc73. gap-analysis reviewed FR-485 and TC-1263 against linked requirements and the public QSL/leaf contracts. An independent Claude Opus semantic audit was consulted; all findings were checked against the source.

## Verdict

**PASS** — No findings in this lens. New FR-485 code/tests are not yet implemented; plan completion: not assessed. The full computed matrix was unavailable in this review; manual AC-to-TC linkage examined.

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
| FND-001 | low | No findings (placeholder) | - |
