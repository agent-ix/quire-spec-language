---
id: SR-1060
title: "Spec review of PR #576 (ADR-029 lifecycle, CLI, plugins, cache and execution backends; FR-275 to FR-299)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@546ff375650521c04e3dd0e1a4a7fb35b660ac93; spec/decisions/ADR-029-lifecycle-cli-provider-plugin-cache-codegen-boundaries.md, spec/functional/FR-275..FR-299, spec/functional/FR-027, spec/functional/FR-100, spec/usecase/US-029, spec/usecase/US-030, spec/tests/TC-755..TC-786, spec/spec.md, spec/tests.md"
review_set: subset
---

## Summary

Ticket: QSL-390. Spec-only PR. Reviewed `git diff origin/main...HEAD` at
546ff375 against the owner rulings recorded on QSL-390 (CLI driver with a
separable core; `analyze` in the core through its certificate checkers only;
in-process engines only, SMT through the driver; plugin rights model deleted,
plugin proofs labelled `trusted`; plugin results never cached; undefined exits
10; backend seam plus JIT, no WASM), the cross-cutting QSL-366 ruling
(an undefined claim evaluation settles REFUTED with cause
`UndefinedEvaluation{where, cause}`), the wave B/C rules, and the QSpec
counterparts on branch spec/wave-b-q8-lifecycle-depth (FR-300, FR-301,
FR-305, FR-306, FR-462, FR-354).

Checked clean: no fixed caps, and depth is never a limit (FR-277, FR-295,
ADR-029 LC-4, EB-4 item 5); no rights model, no WASM, no plugin cache entry
(FR-290, FR-293, ADR-029 RU-4, RU-5); exit table and severity order match
QSpec FR-301 (FR-285, US-029-EX-5); the cache key holds no version string or
path and has no index or ledger (FR-292); every AC has a TC with behaviour
steps; ticket ids sit only in References and Status; STD citations carry the
QSpec FR numbers; `quire validate` on every changed file exits 0 and
`tools/check-index-completeness.sh` passes.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-281 never states the QSL-366 rule: an undefined claim evaluation in an `analyze` engine (EN-1, EN-4, EN-5, zone search) settles `refuted` with cause `UndefinedEvaluation{where, cause}`, replay reproducing the undefined value. ADR-029 OP-2 is silent too, so `analyze` has no stated outcome for an undefined evaluation. | spec/functional/FR-281-analyze-claims-with-in-process-engines.md:61-85, spec/decisions/ADR-029-lifecycle-cli-provider-plugin-cache-codegen-boundaries.md:326-352 |
| FND-002 | medium | FR-286 gives `prove`, `analyze` and `monitor` items "for an undefined item, the label `undefined`", but after QSL-366 a proof item never settles undefined (ADR-013 O-16 Proof column "not produced"; it settles `refuted` + `UndefinedEvaluation`). RU-1's "label undefined" fits evaluation outcomes (`run`/`execute`); applied to proof items it contradicts QSL-366. FR-283 also never says what an undefined clause evaluation over a trace settles. Both rulings are dated 2026-10-01; the team lead needs to reconcile them. | spec/functional/FR-286-serialize-every-outcome-as-one-json-outcome-document.md:36-38, spec/functional/FR-285-map-every-outcome-category-to-one-exit-code.md:42-43, spec/decisions/ADR-029-lifecycle-cli-provider-plugin-cache-codegen-boundaries.md:275 |
| FND-003 | medium | FR-290 says QSpec FR-354 makes every `proved` record carry exactly one of `trusted`, `certificate-checked`, `solver-asserted`. FR-354 on the QSpec branch defines a basis only for SMT proofs (Carcara-checked or solver-asserted) and plugin proofs. So FR-290 gives no basis for a Kani `proved` (the first-party provider in FR-299) or for a third-party compile-time `Provider`, and reuses `certificate-checked`, which FR-354 defines as a Carcara check, for `analyze` certificates without a QSpec counterpart. | spec/functional/FR-290-settle-plugin-results-as-typed-terminal-records.md:56-61 |
| FND-004 | medium | FR-100-AC-10 now requires exit 10 but still names TC-452 as its verification. TC-452 is `✅ Passed locally`, and its test `tc_452_step_5_sum_over_pos_is_sum_out_of_domain_or_completes` (traced to FR-100-AC-10) composes with `undefined_kernel_reasons_render_and_exit_20`, which asserts exit 20. tests.md also lists AC-10 under TC-786, so two TCs claim the AC, and the passing one checks the old behaviour. | spec/functional/FR-100-run-a-named-function-through-the-spine.md:355, spec/tests.md:243, qsl-replay/src/spine/call/tests.rs:785-791 |
| FND-005 | medium | QSL FRs restate QSpec-owned and driver-owned behaviour as normative QSL text. FR-291's settlement table and "The plugin host shall ..." bullets restate QSpec FR-462's failure rules for `quire-plugin-host`, which the driver owns. Every FR-291 AC is a driver integration test. The same pattern appears in FR-292 and FR-293 (driver `quire-cache`, restating QSpec FR-306's cache rules), FR-287-AC-2 to AC-4, FR-296-AC-4 and FR-299. QSL should cite QSpec FR-462 and FR-306 and keep only QSL's own part: terminal record types, the canonical `analyze` form, and the settlement from a reader result. | spec/functional/FR-291-bound-plugin-runs-and-settle-plugin-failures.md:39-79, spec/functional/FR-292-key-cached-results-by-content-identity.md:70-87, spec/functional/FR-293-never-cache-failed-cancelled-timed-out-or-plugin-results.md:42-51 |
| FND-006 | medium | FR-283's Behavior settles a clause with a missing fairness premise in category unsupported, but no AC tests it, and the Outputs table, which lists verdicts and exit codes, has no unsupported row. | spec/functional/FR-283-monitor-a-supplied-trace-offline.md:72-73, spec/functional/FR-283-monitor-a-supplied-trace-offline.md:46-51 |
| FND-007 | low | FR-277-AC-1 exercises only the limits types of `parse`, `check`, `package`, `monitor` and `replay`, but the Behavior binds every lifecycle operation. The limits of `select`, `analyze`, `execute`, `inspect` and `render` have no AC showing that reaching a bound names its field. | spec/functional/FR-277-bound-every-lifecycle-operation-by-caller-limits.md:65 |
| FND-008 | low | FR-288 justifies the manifest content identity in `BackendId` as "binds a result to the manifest that produced it", which is a provenance rationale. Its working consumer is the FR-292 cache key, which plugin results never reach (FR-293). State what the identity does (the cache key, and the producing provider's identity on a `proved` record), not provenance. | spec/functional/FR-288-build-the-registry-from-provider-manifests.md:33-36 |
| FND-009 | low | FR-282 names closure checks (ADR-022 trap re-exploration) as in-core certificate checkers, but its ACs cover only the EN-5 and zone checkers. | spec/functional/FR-282-check-analyze-certificates-in-the-qualified-core.md:21-24 |
| FND-010 | low | ADR-029 says its amendments "are not applied while the owner's hold stands". The owner ruled RU-1 to RU-5 on 2026-10-01, and no hold is recorded on QSL-390. This is stale status text; state what is. | spec/decisions/ADR-029-lifecycle-cli-provider-plugin-cache-codegen-boundaries.md:756-757 |

## Verdict

Not mergeable as it stands. FND-001 is a missing cross-cutting owner ruling
in the `analyze` requirement. FND-002 needs the team lead to reconcile RU-1's
"label undefined" with QSL-366 for proof items. FND-003 to FND-006 are real
gaps or wrong bindings. The rest of the set is consistent with ADR-029 and
the owner rulings: separable core by crate, certificate-checked `analyze`,
in-process engines only, no plugin rights, no plugin caching, undefined at
10, and the backend seam with JIT and no WASM.

## New findings (disposition pass 1)

Reviewed at bc11b32cc7c51954c6f8f8f17c68c7949605bf52 (546ff375..bc11b32c), limited to lines the PR changed.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-011 | medium | New FR-288-AC-4 requires "two manifests that differ only in a tool version member give different `BackendId`s", and the new rationale says "a changed manifest is a cache miss". That makes a tool version part of the backend identity and of the cache key. QSpec FR-306 on main keys the cache by "the provider's backend identity" and puts application version strings outside the key (FR-306-AC-2), and QSpec #176 removed the manifest digest from backend identity. QSL now disagrees with QSpec and builds version tracking into identity. Make `BackendId` the backend identity only, as QSpec FR-290 and FR-306 state, and replace AC-4's second clause with the matching test (a version-only change leaves `BackendId` and the key unchanged). | spec/functional/FR-288-build-the-registry-from-provider-manifests.md:33-37; spec/functional/FR-288-build-the-registry-from-provider-manifests.md (FR-288-AC-4) |

## New findings (disposition pass 2)

Reviewed at 6991936fa7a00ef024f5bff70520b5cf3adf28d4, the PR's own delta against merge base 0431a38f.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-012 | low | The rewritten ADR-011 S6c row keeps EN-1's `ModelCheckOutcome` and `ReplaySource::ModelTrace`, but drops two pieces of the merge base's ADR-018 text. One is that a completed search horizon, an undecided run and a stopped run settle the item's terminal record at S6c. The other is the citations of the run by the orchestrating driver (T-13) and of ADR-018 PC-1 to PC-5 and LA-3 for the layer-6 check and settlement. E10, E11 and A keep ADR-018's and ADR-027 PS-1's text. Restore the S6c sentence and citations beside #576's native-engine text. When W4 rebases, main's ADR-020 §9 amendments to S6c and E10 (refinement items), absent at this merge base, must also be kept. | spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md:202 |

## Dispositions

Round 1, reviewed at bc11b32cc7c51954c6f8f8f17c68c7949605bf52.

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | bc11b32c: FR-281 and ADR-029 settle an undefined claim evaluation `refuted`, `UndefinedEvaluation`, with AC-7. |
| FND-002 | fixed | bc11b32c: FR-286 keeps `undefined` off proof items; FR-283 settles an undefined clause as a violation. |
| FND-003 | fixed | bc11b32c: FR-290's producer table gives every `proved` producer its basis, Kani on the qualified path with none. |
| FND-004 | fixed | bc11b32c: FR-100-AC-10 is verified by TC-786 only; TC-452 now traces AC-7 to AC-9. |
| FND-005 | fixed | bc11b32c: FR-291, FR-292 and FR-293 keep only QSL's record types and categories. |
| FND-006 | fixed | bc11b32c: FR-283 has an unsupported Outputs row (exit 21) and AC-6. |
| FND-007 | fixed | bc11b32c: FR-277-AC-1 covers all eleven operations' limits. |
| FND-008 | fixed | bc11b32c: The provenance rationale is gone; see FND-011 for the identity it now states. |
| FND-009 | fixed | bc11b32c: FR-282-AC-4, AC-5 and AC-8 test the trap, EN-1 closure and product-closure checkers. |
| FND-010 | fixed | bc11b32c: The hold sentence is gone; ADR-029 records the 2026-10-01 rulings as applied. |

Round 2, reviewed at 6991936fa7a00ef024f5bff70520b5cf3adf28d4.

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-011 | still-open | FR-288 and TC-772 are identity-only, but ADR-029 PL-5 (lines 500-504) still puts the manifest content identity in `BackendId` with the provenance rationale. FR-288 cites ADR-013 O-19, whose `backend` member is `identity` plus `manifest_digest` and 'also pins the tool'. Fix PL-5, and amend O-19 through ADR-029's amendments (with ADR-012 Q4, FR-075 and TC-433, which carry `manifest_digest`) so the identity-only rule has one source. |

Round 3, reviewed at 3d95c1ed6d058ea51de4b1a7ca276ce0fdcfeeca (rebuilt on origin/main e03e6b1a).

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-011 | fixed | 3d95c1ed: ADR-029 PL-5's `BackendId` is the backend identity alone, verbatim; FR-288-AC-4 expects a version-only change to keep it; no `manifest_digest` on main. |
| FND-012 | fixed | 3d95c1ed: ADR-011 S6c restores ADR-018's settle sentence and T-13, EN-1, LA-1, PC-1 to PC-5, LA-3 citations; ADR-020 §9 kept in S6c and E10. |
