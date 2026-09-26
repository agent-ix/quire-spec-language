---
id: SR-648
title: "QSL-271 integrity review of spine run"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language@3c7c0a8bb365b9ce460dfda97a037682a5c94100; spec/functional/FR-100-run-a-named-function-through-the-spine.md; spec/test-cases/TC-450-cli-run-routes-a-program-by-its-declared-edition.md; spec/test-cases/TC-451-spine-run-binds-arguments-and-maps-outcomes-to-exit-codes.md; spec/test-cases/TC-452-spine-run-entry-lives-in-qsl-replay.md; spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md; spec/spec.md; spec/tests.md; unchanged: spec/functional/FR-001-read-exact-source.md, spec/functional/FR-026-run-standalone-native-workflow.md, spec/functional/FR-027-export-compiled-native-package.md, spec/functional/FR-098-execute-a-replay-request.md, spec/decisions/ADR-014-temporal-trace-and-boundedness-architecture.md, spec/test-cases/TC-105-standalone-package-export.md, qsl-replay/src/execute.rs; quire-specification FR-301 at origin/main (external)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-100
    type: reviews
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: reviews
---

## Summary

Ticket: QSL-271 (PR agent-ix/quire-spec-language#458). This review checks
the diff for consistency with FR-001, FR-026, FR-027 (AC-8 included),
FR-098, ADR-011, ADR-014 and QSpec FR-301.

Consistent:
- FR-301's six codes. Completed 0, refused 20 or 21, undefined 20 and
  incomplete 22 all fall within 0/10/20/21/22/30. Exit 30 for output failure
  and quiet broken pipes match FR-027.
- FR-027's edition reader and the placement of `unknown_edition`.
- FR-027's model and `libraries` intake, and the refusal stages and codes
  of spine compile.
- FR-098's canonical integer rule and its `WrongValueKind` placement: before
  the call for a wrong kind, and at S6a admission for a value out of domain
  (qsl-replay/src/execute.rs).
- FR-098's single-segment name lookup. Replay's `select` refuses
  multi-segment names (execute.rs:518-520).
- The four FR-001 labels.
- The ADR-011 §5 amendment and OQ-1.
- ADR-014: its accounting counters are the ten `ScalarLimits` fields, so
  "the other nine" is right.

Verdict: changes requested (one medium, three low).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-026 is not amended, and its text now contradicts FR-100 for `1-draft` sources. FR-026's Description says run "execute[s] the selected clause against verified runtime artifacts". Its closed request "names ... program source with complete authored clause bindings, snapshot/invocation file selections and an execution selection" and has no `call` member. Its exit contract says "exit 0 means completed true; 10 means completed false". FR-027 carries its own edition routing, but FR-026 has no pointer to FR-100. Failure scenario: a reader or implementer follows FR-026 for native-run/1 decoding. They require `selection` and `snapshots`, reject `call` as an unknown field, and do this before the edition is read, so every `1-draft` request refuses at decode. Or they apply exit 10 to a completed `false`, which the leader has ruled is exit 0. Amend FR-026 so its Description, Inputs and exit contract scope themselves to `0-draft` and route `1-draft` to FR-100, as FR-027 does for compile. | spec/functional/FR-026-run-standalone-native-workflow.md:19, 23-27, 57-60; spec/functional/FR-100-run-a-named-function-through-the-spine.md:44-68, 93-95 |
| FND-002 | low | TC-450 step 2 takes its oracle from "the native run results TC-105 fixes". TC-105 is FR-027's compile export test ("Export and reread exact standalone compiler output") and fixes no run result. The `0-draft` native-run results are fixed by TC-103 and TC-104 (FR-026-AC-1 to AC-5, tests/it/standalone.rs). Failure scenario: the implementer compares against a compile artifact, or finds no baseline and records the current output, which makes the "unchanged" check circular. Cite TC-103 and TC-104, or name a golden file. | spec/test-cases/TC-450-cli-run-routes-a-program-by-its-declared-edition.md:27-28 |
| FND-003 | low | The stale QSL-5 text in ADR-011 was left in place. §5 still says "Until M-6c/QSL-5, CLI `compile` reaches spine `compile` through a native-compile/1 request ...; the operand form lands with QSL-5 (Ruling 2026-09-24)". The bullet also opens with "Spine `compile` takes complete-V1 source: `compile <identity> <revision> <path>`". OQ-2 repeats "The request format is `compile <identity> <revision> <path>`". QSL-5 is closed, and the operand form did not land. The PR edits the next bullet in the same list. Failure scenario: a reader takes the operand form as the current contract, or as pending QSL-5 work, and builds or waits for it. Remove the QSL-5 clause and the operand-form statement in §5 and OQ-2, and state the native-compile/1 route as the form. | spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md:652-665, 1300-1303 |
| FND-004 | low | Two small divergences from FR-098 are not stated. (a) A function whose result type is not supported refuses `unsupported_construct` (exit 21) in FR-100. Replay's analogous `NotAPredicate` refuses `invalid_runtime_input` (exit 20; qsl-replay/src/execute.rs:183-188), and FR-100 says it follows FR-098. (b) FR-100's Dependencies cite FR-001 (labels) and FR-038 (integer spelling), but its `relationships` do not list either, so the dependency graph misses both edges. Failure scenario: a later edit aligns one of the two codes without knowing the difference was deliberate. Add one sentence saying the difference is deliberate, and add `references` edges for FR-001 and FR-038. | spec/functional/FR-100-run-a-named-function-through-the-spine.md:4-19, 109, 169-183; qsl-replay/src/execute.rs:183-188 |

## Dispositions

Disposition pass at `agent-ix/quire-spec-language@8c786c48fcd7b6fdf4bc56799b4bf9db1fa2ce48` (fix commit `8c786c48`, "QSL-271 spec: fix SR-646 to SR-649 findings on spine run"). Each outcome was re-checked against the spec at that head, not taken from the commit message.

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed 8c786c48 | FR-026's Description, exit contract and Inputs now scope themselves to `0-draft` or no edition and route `1-draft` to FR-100. A Behavior SHALL admits FR-100's members at decode and applies the member rules for the edition after the header is read. An FR-100 `references` edge was added. "No edition" routes native, as `Edition::of` does in code (src/command.rs:398-403). |
| FND-002 | fixed 8c786c48 | TC-450 step 2 now cites the TC-103 and TC-104 requests and assertions in `tests/it/standalone.rs`. |
| FND-003 | fixed 8c786c48 | ADR-011 §5 now states the native-compile/1 route (FR-027) with no operand form and no QSL-5 clause. OQ-2 now says "a native-compile/1 request routed by the program's declared edition", amended 2026-09-26. Neither `compile <identity>` nor "lands with QSL-5" remains anywhere in spec/. |
| FND-004 | fixed 8c786c48 | FR-100 states the `unsupported_construct` versus `NotAPredicate` difference as deliberate, with a reason. `references` edges to FR-001 and FR-038 were added. |
