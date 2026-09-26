---
id: SR-647
title: "QSL-271 EARS review of spine run"
type: SpecReview
analysis: ears-conformance
scope: "agent-ix/quire-spec-language@3c7c0a8bb365b9ce460dfda97a037682a5c94100; spec/functional/FR-100-run-a-named-function-through-the-spine.md; spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md (§5 amendment)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-100
    type: reviews
---

## Summary

Ticket: QSL-271 (PR agent-ix/quire-spec-language#458). This review checks
FR-100's requirement statements against the EARS patterns.

Clean: the Description's event-driven lead ("When an author invokes ..., the
run command shall route ...") conforms. So does the unwanted-behaviour
statement ("If the program declares `1-draft`, then the run command shall
refuse ..."). The eight `qsl_replay::spine::run shall ...` Behavior bullets
are well-formed ubiquitous statements with one actor each, and none uses a
weak modal.

Verdict: pass with low findings.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The unknown-edition refusal and the refusals at stage `call` (`missing_declaration`, `unsupported_construct`, `invalid_runtime_input` for joins) appear only as rows of an Outputs table. No "If <trigger>, then <system> shall <response>" statement carries them. FR-027 states the same unknown-edition case as a Behavior SHALL. Failure scenario: someone tracing requirements to SHALL statements finds no normative statement for AC-2's second sentence or for AC-5. Add If/then statements, or state once that the Outputs refusal table is normative. | spec/functional/FR-100-run-a-named-function-through-the-spine.md:98-112, 116-157 |
| FND-002 | low | The `0-draft` route has no SHALL. The Description says "A `0-draft` source runs its selected clause through native run ..., unchanged", and no Behavior bullet repeats it. AC-2's "same bytes it wrote before this change" therefore has no normative statement behind it. Add "If the program declares `0-draft`, the run command shall run it through FR-026 native run". | spec/functional/FR-100-run-a-named-function-through-the-spine.md:28-30, 162 |

## Dispositions

Disposition pass at `agent-ix/quire-spec-language@8c786c48fcd7b6fdf4bc56799b4bf9db1fa2ce48` (fix commit `8c786c48`, "QSL-271 spec: fix SR-646 to SR-649 findings on spine run"). Each outcome was re-checked against the spec at that head, not taken from the commit message.

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed 8c786c48 | Behavior now has If/then SHALLs for the unknown edition, a malformed call or `work_units`, a bad `function`, a name that resolves to nothing, an unsupported result, the join refusals and `WrongValueKind`. |
| FND-002 | fixed 8c786c48 | "If the program declares `0-draft` or no edition, then the run command shall run the program through FR-026's native run" was added, and AC-2 now points at the TC-103 and TC-104 results. |
