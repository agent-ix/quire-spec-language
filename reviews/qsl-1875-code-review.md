---
id: SR-2026111117221900000001
title: "code review of PR 2680 await anchor and channel ends read views"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; qsl-package/src/protocol_view.rs, qsl-package/src/emit/tests/protocol_view.rs, spec/functional/FR-519-publish-the-protocol-read-surface-the-quire-protocol-link-layer-imports.md, spec/test-cases/TC-1314-publish-the-protocol-read-surface-the-quire-protocol-link-layer-imports.md"
review_set: subset
---

## Summary

Ticket: QSL-1875 (with QSL-1872, channel half). PR agent-ix/quire-spec-language#2680 at head a3670ee52, diff origin/main...HEAD (merge base e35d1ac6c) over 4 files. The rust-review lane is folded in. Gates on the head: `cargo fmt --check -p qsl-package` passes; focused `cargo test -p qsl-package --lib protocol_view` result is recorded in the Verdict.

GitHub reports the PR as CONFLICTING with main. Main moved to 91ada09a7, which merged PR #2679 (QSL-1877, with QSL-1875 and the QSL-1872 channel half). That PR already wrote FR-519-AC-18 and AC-19 and TC-1314 rows 18 and 19, with a different API from the one this PR implements.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The PR implements and re-specifies an API that conflicts with FR-519 as merged on main by #2679. Main specifies `await_anchor(node) -> Result<AwaitAnchor<'_>, ViewRefusal>` returning a public struct `AwaitAnchor { kind: AwaitAnchorKind, target: &CheckedNodeId }` (AC-18 requires `AwaitAnchor` and `AwaitAnchorKind` public), and `channel_endpoints(channel) -> Result<ChannelEndpoints<'_>, ViewRefusal>`. The PR returns a tuple `(AwaitAnchorKind, &CheckedNodeId)`, names the channel function `channel_ends` and its struct `ChannelEnds`, and rewrites AC-18/AC-19 and TC-1314 rows 18/19 with its own text. The spec files conflict textually, and once resolved to main's text the code no longer implements FR-519. Fix: rebase on main, drop this PR's FR-519 and TC-1314 edits in favour of main's, return a public `AwaitAnchor<'n>` struct, and rename to `channel_endpoints` / `ChannelEndpoints`. | qsl-package/src/protocol_view.rs:861-874; qsl-package/src/protocol_view.rs:944-967; spec/functional/FR-519-publish-the-protocol-read-surface-the-quire-protocol-link-layer-imports.md:102-110; spec/functional/FR-519-publish-the-protocol-read-surface-the-quire-protocol-link-layer-imports.md:191-192 |
| FND-002 | medium | The `MalformedMember` refusal of the channel view has no test. FR-519-AC-19 on main requires that with `from` set to a text term the view refuses `MalformedMember` naming `from` (TC-1314 row 19). The test removes `from`, `to` and `carries` but never sets one to a non-reference, so changing `single` to return `MissingMember` for a non-reference would still pass. Fix: add the text-term `from` case to the AC-19 test. | qsl-package/src/emit/tests/protocol_view.rs:1777-1786 |
| FND-003 | low | The message type is not checked against the in-memory channel. AC-19 says the ids equal the `from`, `to` and `carries` the checked clause holds, but the test only asserts that `message_type` names a node whose form ends with `type`. A view returning another type node of the package would pass. Fix: compare `message_type` with the `carries` reference of the channel node, or with the type node the in-memory `carries` lowers to. | qsl-package/src/emit/tests/protocol_view.rs:1770-1775 |

## Verdict

FAIL. FND-001 blocks merge: the PR conflicts with main and its API differs from FR-519 as merged. FND-002 and FND-003 should be fixed in the same round.

What is sound: `await_anchor` and `causal_relation` now share one read of `after`, so the `AwaitAnchor` edge and the public view cannot disagree, and the removed `None => malformed` arm is now covered by `from_wire` inside `await_anchor`. Both functions reuse the module's `required`/`single`/`group_member` helpers and refuse with the same `ViewRefusal` shapes as `delivery` and `queue_capacity`. Doc comments match the behaviour. No panics on input, no unsafe, no new dependencies. The AC-18 test covers both kinds and the missing `after`, missing `kind`, missing `target` and unknown-word refusals. Both tests carry `#[trace]` tags. Not checking `semantic_form` matches the sibling node readers. Focused `cargo test -p qsl-package --lib protocol_view` passed on the head: 23 passed, 0 failed.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Verified in b7e563872: rebased on main, PR spec edits dropped, `await_anchor` returns public `AwaitAnchor<'n> { kind, target }`, channel view renamed `channel_endpoints` / `ChannelEndpoints`; PR is MERGEABLE. |
| FND-002 | fixed | Verified in b7e563872: the AC-19 test sets `from` to the text `delivery` term and asserts `MalformedMember` naming `from`. |
| FND-003 | fixed | Verified in b7e563872: the AC-19 test asserts `message_type` (and sender, receiver) equal the channel node's `carries` (`from`, `to`) reference. |
