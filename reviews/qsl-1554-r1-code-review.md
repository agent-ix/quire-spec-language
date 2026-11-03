---
id: SR-2207001
title: "Code review of PR #2207: await after a compensation template (QSL-1554)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@4314c1cad; git diff f385c801e...4314c1cad (PR #2207, branch qsl-1554-await-after-compensation): qsl-semantics/src/check/lowering/protocol.rs, qsl-semantics/src/check/lowering/protocol/controls.rs, qsl-package/src/emit/tests/protocol_view.rs; ticket QSL-1554"
review_set: subset
date: '2026-11-03'
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-519
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-1314
    type: references
---

## Summary

Ticket: QSL-1554. PR: quire-spec-language#2207, head 4314c1cad, merge base f385c801e. Method: code-review with the rust-review lane. A trial merge onto origin/main f64bfc001 is textually clean; main is 6 commits ahead and none of them touches the three changed files.

The root cause the coder names is correct. Before the change, `Controls::build_order` pushed an `await`'s resolved anchor as a key dependency. For `await ... after <template>` that anchor is a `CompensateTemplate` declaration, not a control, so `tree.control(name).is_none()` returned `UncheckedProtocolFact`. The `Await` arm of `control_arguments` also always wrote `after.kind = "event"`.

The fix is sound:

- `template_of` returns a template index only when the anchor's declaration kind is `CompensateTemplate`. Every other anchor kind takes the unchanged `None` path, so receive, effect and event-kind await anchors are unaffected.
- `build_order` makes the await depend on the template's forward effect and, when present, its commit control. Both are leaves: S3 admits only an `Effect` for `compensate-for` and only a `Commit` for `compensate-commit` (anchors.rs:34-36). An effect's own key depends only on its `attempt`, which anchors nothing. So the new dependency edges cannot close a key cycle that S3's causal-cycle check missed. A forward effect that follows the await is refused earlier by S3, through the `AwaitAnchor` edge `enter(effect) -> enter(await)` (graph.rs:340-356).
- The template is keyed once. `run_nodes` memoizes it in `compensations` through `Entry::Vacant`, so a second await on the same template reuses the key. `compensation_nodes` then reuses every built key and builds only the rest, so there is one `insert_node` and one `Declaration` occurrence per template. The template reads only `checked.effect` and the commit control from the partial key map, and both are final by then, so the content (and therefore the content-addressed key) is the same as when the template is built late. `compensation_nodes` still returns `sorted(keys)`, so its consumer (the clause node) is unchanged.
- The await writes `after = {kind: "compensation", target: <template key>}` (QSpec FR-466 await layout). The checker can never write kind `compensation` with a control target, because `template_of` requires the template kind. TC-948 AW7 (kind `compensation` naming a control) is therefore a reader-only vector that this emitter cannot produce. The reader side is unchanged.

The rewritten e2e test `an_await_after_a_compensation_is_anchored_at_the_forward_effect` now runs real source (`await WaitEvent after Refund`) through check, emit, IR's reader and QSL's I2 reader. It asserts `kind == "compensation"`, `target == reference(Refund)`, and exactly one `AwaitAnchor` edge from `Applied`'s enter port to the await's enter port. By inspection it fails under each requested mutation. Writing kind `event` fails the kind assertion, and IR's reader would also refuse a template as an event-kind target. A wrong target fails the target assertion. Dropping the edge in `protocol_view::awaiting` fails the anchors assertion. The old test hand-edited the emitted node to fake the same shape, and the new one covers everything the old one did plus the checker path, so no coverage was lost.

The tags hold. FR-519-AC-1/TC-1314 row 1 (the causal relation) carries the old test's tags forward. FR-466-AC-6 and TC-948 AW6 ("an await is admitted after a compensation"; "admitted; the await reads Refund's activation") are QSpec ids. Tagging QSpec ids follows existing practice (protocol_requirements.rs:410 tags FR-464-AC-13/TC-946).

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| Lowering::run_nodes template memo | qsl-semantics/src/check/lowering/protocol/controls.rs:200-273 | examined |
| control_arguments Await arm | qsl-semantics/src/check/lowering/protocol/controls.rs:457-478 | examined |
| Controls::template_of | qsl-semantics/src/check/lowering/protocol/controls.rs:1007-1021 | examined |
| Controls::build_order | qsl-semantics/src/check/lowering/protocol/controls.rs:1032-1088 | examined |
| compensation_nodes / compensation_node split | qsl-semantics/src/check/lowering/protocol.rs:1356-1735 | examined |
| RunInputs wiring | qsl-semantics/src/check/lowering/protocol.rs:1016-1043 | examined |
| e2e test | qsl-package/src/emit/tests/protocol_view.rs:484-555 | examined |
| S3 AwaitAnchor edge and cycles | qsl-semantics/src/check/protocol_clause/graph.rs:333-366 | context_only |
| anchor admitted kinds | qsl-semantics/src/check/protocol_clause/anchors.rs:28-45 | context_only |
| view AwaitAnchor derivation | qsl-package/src/protocol_view.rs:749-783 | context_only |
| QSpec FR-466 await layout, FR-466-AC-6, TC-948 AW5-AW8, FR-464 derived causal relation | quire-specification origin/main | context_only |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | Three comments are stale now that an await's anchor can be a template. The `Controls::anchored` doc says it holds "the control" each await is anchored at. The `build_order` doc says the await is keyed after "the control it is anchored at", not the template's effect and commit. The anchor loop's "A compensation is not written yet (`compensate` templates are carried by the protocol clause)" is no longer true, since `run_nodes` may now write a template. | qsl-semantics/src/check/lowering/protocol/controls.rs:112,169-170,1032-1034 |
| FND-002 | low | The new `compensation_node` is `pub(super)`, but its only callers are in `protocol` and its child module `controls`. Its sibling `protocol_member_ref` is private and is called from `controls` already. The signature also copies the fully qualified `std::collections::BTreeMap<crate::check::protocol_clause::ProtocolNodeId, NodeRef>` although both names are imported at protocol.rs:27 and :49. | qsl-semantics/src/check/lowering/protocol.rs:1398-1408 |

## Verdict

PASS. The fix is correct, deterministic and minimal (most of the protocol.rs diff is the de-indentation of one loop body into `compensation_node`), and the e2e test is value-asserting. The two findings are low-severity hygiene and do not block merge. On the trial merge with origin/main f64bfc001 (merge 6b81d9b3c), fmt, `clippy --workspace --all-targets` and `--all-features` are clean. The workspace test run was stopped before it finished and is deferred to the batch gate with #2204.
