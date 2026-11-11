---
id: SR-2026111117221900000002
title: "gap analysis of PR 2680 await anchor and channel ends read views"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; FR-519-AC-18, FR-519-AC-19, TC-1314 rows 18-19, qsl-package/src/protocol_view.rs, qsl-package/src/emit/tests/protocol_view.rs"
review_set: subset
---

## Summary

Ticket: QSL-1872 (partial) and QSL-1875. PR agent-ix/quire-spec-language#2680 at head a3670ee52, scoped to the diff. Plan completion: not assessed. Requirement-to-test-to-code check, by hand against the diff: FR-519-AC-18 is tagged on `the_await_anchor_is_the_kind_and_target_the_node_carries` and FR-519-AC-19 on `a_channels_ends_and_message_type_are_the_nodes_it_references`, both with TC-1314. Each public function added (`await_anchor`, `channel_ends`, `AwaitAnchorKind`, `ChannelEnds`) is owned by FR-519. No stub or placeholder code. The AC gaps against main's FR-519 text are in the code-review SR (FND-001, FND-002).

QSL-1872 asked for four facts. The PR delivers sender, receiver and message type and deliberately leaves out role internality. That partial delivery is correct. QSpec FR-217's Status section says "Current Spec Language has no internality field ... a versioned producer/binding contract must carry the required values". QSL FR-384-AC-7 and QSpec FR-465-AC-8 refuse an `internal`/`external` member on a role node. A QSL view of internality would therefore have nothing to read. The spec documents this both in the PR's FR-519 text and in main's FR-519 text from #2679 ("A role carries no internality member ... so no view function returns one"; "`ChannelEndpoints` has no internality field").

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The internality residue of QSL-1872 has no owner ticket. The PR body says it needs an owner decision on the producer shape. The QSL triage comment on QSL-1872 says QSpec contradicts itself: FR-465 and FR-465-AC-8 say a role has no internality member, while FR-050 and FR-211 say every static role carries exactly one. No Linear ticket carries that ruling. QSL-1872 is the only record, and the branch `qsl-1872-1875-read-view` links it to this PR, so merging can move QSL-1872 to Done and drop the residue. quire-protocol FR-151-AC-7 then stays untagged with no open ask. Fix: before merge, file the QSpec ticket for the internality producer/binding contract (FR-050/FR-211 against FR-465), link QSL-1872's remainder to it, and keep QSL-1872 open or rescoped so that merging does not close it. | spec/functional/FR-519-publish-the-protocol-read-surface-the-quire-protocol-link-layer-imports.md:107-109 |

## Verdict

PASS WITH FINDINGS for coverage: both new ACs have tagged tests and every new public item has an owning requirement. FND-001 is tracker hygiene for the deliberate partial, which is itself correct and documented in the spec.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | QSL-1872 is kept open and rescoped to the internality residue (code lead comment on QSL-1872, 2026-11-11), and the PR body at b7e563872 carries `Remaining work: QSL-1872 role internality` with no closing keyword. |
