---
id: SR-16220
title: "Spec review of quire-spec-language PR #1316: STD-350, PrepareLimits, UnsupportedEntry, CallFailure::Unsupported and the parity generator"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@82acd324a168d637cd18615a82e5415fbf8520f7; PR #1316 (git diff origin/main...82acd324): spec/functional/FR-255-name-the-setting-that-raises-a-reached-limit.md, FR-279-execute-a-checked-entry-as-a-library-operation.md, FR-285-map-every-outcome-category-to-one-exit-code.md, FR-294-run-every-execution-backend-behind-one-seam.md, FR-295-return-interpreter-outcomes-from-jit-and-aot.md, FR-401-host-the-evaluation-interface-in-the-quire-eval-leaf.md, FR-402-differentially-test-the-no-std-evaluator-against-the-std-reference.md, FR-403-implement-the-no-std-evaluator-in-quire-eval.md, spec/test-cases/TC-760, TC-778, TC-779, TC-780; read for context: open PR #1323 (STD-369) head FR-294/FR-279/FR-285, PR #1306 head FR-255; QSL FR-286; agent-ix/quire-specification main FR-271, FR-272"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-294
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-295
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-279
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-255
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-401
    type: reviews
---
# SR-16220: Spec review of quire-spec-language PR #1316: STD-350

## Summary

Ticket: STD-350. Reviewed at the current head 82acd324, not the 76a5853d named in the dispatch: the author pushed "CallFailure::Unsupported carries an unsupported entry". Sub-analyses folded in: integrity, EARS, criterion strength (manual, no Jev), cross-PR consistency, id uniqueness, scope-deferral check. Examined: FR-294's prepare-limits and unsupported-entry sections, Behavior, and FR-294-AC-5 to AC-7; FR-295 items 5-6, the parity program generator, Behavior and FR-295-AC-1 to AC-3, AC-5 and AC-6; FR-279 Outputs, Behavior and FR-279-AC-4; the FR-285 prose and AC-3; FR-401's catalog row, the `CallFailure::Unsupported` paragraph and FR-401-AC-11; the FR-402/FR-403 comparison rows; FR-255's rows; and the TC-760/778/779/780 steps. Main is an ancestor of the head.

The author's three return items are handled. The `CallFailure` unsupported arm is decided: `CallFailure::Unsupported(UnsupportedEntry)` from `PreparedEntries::call`/`evaluate` and `execute`, before any meter charge, exit 21, in the `quire-eval` leaf. The FR-255 text is now byte-identical to #1306's, so the two merge cleanly in either order. The QSpec FR-272 `invalid_quire_context` row is #1306's concern, added on the QSpec branch `spec/std-349-invalid-quire-context-cause`, and is not in this PR's diff.

## Verdict

**NOT MERGEABLE.** FND-001: open PR #1323 (STD-369) specifies the same seam types differently and reuses two of this PR's AC ids. FND-002: the PR body still declares an open decision (standing rule). FND-003 to FND-005 are medium. FND-006 is low.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Open PR #1323 (STD-369, "FR-294 seam types, unsupported entries inside Prepared, CallFailure::Unsupported") contradicts this PR on the same surfaces. Field and setting: `work_units`/`prepare.work_units` here, `compile_work_units`/`prepare.compile_work_units` there. Shape: `UnsupportedEntry { entry: NodeKey, construct: UnsupportedConstruct }` here, `{ backend, cause: NotSupplied \| Untranslatable { location } }` there. `PrepareFailure::Unsupported` exists here, while #1323 says `PrepareFailure` "has no per-entry variant". #1323 also adds `CallFailure::Limit`, which this PR does not. Both PRs add FR-294-AC-5 and FR-279-AC-4. Whichever merges second breaks FR-294. The lead must assign one owner, then reconcile and renumber. | spec/functional/FR-294-run-every-execution-backend-behind-one-seam.md:58,71-96,156; spec/functional/FR-279-execute-a-checked-entry-as-a-library-operation.md:71 |
| FND-002 | high | scope-deferral: the PR body still says "Open, not decided here: how `execute`'s ... carries an unsupported entry ... and what `PreparedEntries::call` returns ... Both need a cross-repo `CallFailure` decision." The head now decides both (FR-294, FR-279, FR-401). The body names an open decision, which the standing rule forbids, and contradicts the spec. Update the body. RES-56 is outside the ticket, and FR-295 binds either generator choice, so it is not a deferral. | PR #1316 body, "Open, not decided here" |
| FND-003 | medium | A serialized `UnsupportedEntry` is given only its catalog code `unimplemented_capability`. QSpec FR-272 has no `unimplemented_capability` row, so the diagnostic's cause tag and payload keys (`entry`, `construct`) in the FR-286 document are unstated, and FR-294-AC-5 and FR-279-AC-4 cannot check them. #1306 handled the same gap for `invalid_quire_context` with a QSpec FR-272 row. Specify the cause and payload (QSpec row, or QSL-local spelling with rationale). | spec/functional/FR-294-run-every-execution-backend-behind-one-seam.md:90-95 |
| FND-004 | medium | The parity generator's "default seed is a fixed published constant", but no value is given, and FR-295-AC-1, AC-2, AC-5 and AC-6 all run "at the default seed". Two implementations can choose different seeds and corpora and both pass. State the seed value, or the file that publishes it, in FR-295. | spec/functional/FR-295-return-interpreter-outcomes-from-jit-and-aot.md:64-66 |
| FND-005 | medium | FR-255 here also adds #1306's `render.output_bytes` row, owner entry and stage-limit cell (STD-349). If this PR merges before #1306, FR-255 names `qsl_inspect::RenderLimits`, which FR-298 on main does not define. Keep to `prepare.*` and let #1306 add `render.*`. | spec/functional/FR-255-name-the-setting-that-raises-a-reached-limit.md:80,121,274 |
| FND-006 | low | `UnsupportedConstruct::Operation { operation, location }` has no AC. FR-294-AC-5 exercises only `ExpressionForm` and `ValueType`, so a backend that never produces `Operation` passes. Add an operation case to AC-5. | spec/functional/FR-294-run-every-execution-backend-behind-one-seam.md:82,156 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | fc63c46b: this branch merges #1323's 7875f6ca and takes its FR-279, FR-285, FR-294, FR-296 and TC text unchanged. The two PRs now agree on `PrepareLimits { compile_work_units, code_bytes }`, `UnsupportedEntry { backend, cause }`, `PrepareFailure` (`Limit`, `Cancelled`, `Fault`, no per-entry arm) and `CallFailure::{Unsupported, Limit(PrepareLimitReached)}`. FR-294-AC-5 and FR-279-AC-4 are #1323's rows, verbatim, so they are one definition and not a collision. #1323 has since moved to efcddb17 (a 3-line FR-279 Behavior edit). `git merge-tree` of fc63c46b with efcddb17 is clean. |
| FND-002 | fixed | fc63c46b: the body no longer has an "Open, not decided here" section. RES-56 stays named as an engine choice outside the ticket, and FR-295 binds either choice. |
| FND-003 | fixed | fc63c46b FR-401: "Its QSpec FR-272 cause is `backend-not-supplied` for `NotSupplied` and `untranslatable-entry` for `Untranslatable`; the diagnostic carries the payload key `backend` ... and for `untranslatable-entry` also `location`". FR-401-AC-11 checks the serialized diagnostic. QSpec #399 adds the matching `unimplemented_capability` row (FR-272-AC-25), and the body says to merge with or after #399. |
| FND-004 | fixed | fc63c46b FR-295: "The gate's default seed is `0x51534C2D4A495431` (the ASCII bytes `QSL-JIT1` read as a big-endian `u64`)". Verified: 51 53 4C 2D 4A 49 54 31 is `QSL-JIT1`. FR-295-AC-5 asserts the value. |
| FND-005 | fixed | fc63c46b: FR-255 carries only `prepare.*` lines: the FR-003 owner line, two rows at the table end and a new Behavior-1 row. Merging with #1306 is clean. |
| FND-006 | fixed | fc63c46b: `UnsupportedConstruct` and its `Operation` variant no longer exist, so no untested variant remains. The narrowing this causes is FND-007. |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-007 | high | scope-deferral (narrowing): STD-350 item 2 decided `UnsupportedConstruct`, closed to `ExpressionForm`, `ValueType` and `Operation`, "each naming the construct and its `Location`". The head's cause is only `Untranslatable { location }`. An unsupported entry, the serialized `untranslatable-entry` diagnostic and the parity-gate report now name a location and not what the backend cannot translate. Adopting #1323's shape dropped the construct kind STD-350 specified. Add the construct (form, value type or operation) to `Untranslatable`, FR-401's payload and QSpec #399's `untranslatable-entry` keys. Otherwise the owner has to change STD-350 item 2. A lead default cannot narrow it. | spec/functional/FR-294-run-every-execution-backend-behind-one-seam.md:52 |
| FND-008 | medium | `PrepareLimits` names `compile_work_units` and `code_bytes` as "the translation work and the generated code size one `prepare` may use" and no counter. What one compile work unit is, what `code_bytes` counts, when each charge happens and whether `&Cancel` is polled at each charge are all unstated (the earlier head had "one unit per checked node the backend translates, charged before it translates the node", the byte length of each generated block, and a poll at every charge). FR-294-AC-5 uses a test backend that charges "one compile work unit per entry", so a real JIT and AOT can count differently and both pass. State the counters, the charge points and the cancel poll. | spec/functional/FR-294-run-every-execution-backend-behind-one-seam.md:49,120 |
| FND-009 | low | The PR body's "What this PR specifies" still says "#1306 (STD-349) carries the identical FR-255 diff together with its `render.output_bytes` row". That contradicts the body's later FR-255 section and both heads, which edit disjoint lines. Update the bullet. | PR #1316 body |
