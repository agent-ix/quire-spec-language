---
id: SR-2026111204015000000001
title: "code review of PR 2684 FR-519 choice_visibility accessor"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; qsl-package/src/protocol_view.rs, qsl-package/src/emit/tests/protocol_view.rs"
review_set: subset
---

## Summary

Ticket: QSL-1890. Code review with the rust-review lane of PR 2684 at 7020b291b against origin/main. The PR adds the public `ChoiceVisibility` struct and `choice_visibility(node) -> Result<ChoiceVisibility, ViewRefusal>` to `qsl_package::protocol_view`, plus one TC-1314 row-20 test. Gates are recorded in the Verdict.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The `# Errors` doc and the FR-519 prose name only `MalformedMember` naming `operator` for a node that is not a `choice`. A node that is not an admitted profile-operator application (for example a role or a value node) reaches `control_operation`'s `MissingMember` naming `operation` or `operator`, which is not stated. The function behaves correctly; the documented refusal list is incomplete. | qsl-package/src/protocol_view.rs:1024-1037; spec/functional/FR-519-publish-the-protocol-read-surface-the-quire-protocol-link-layer-imports.md:142-151 |
| FND-002 | low | `member: if member == "owner" { "owner" } else { "visible" }` is redundant. The loop runs over the array `["owner", "visible"]`, so `member` is already `&'static str` and can be used directly. | qsl-package/src/emit/tests/protocol_view.rs:1870-1880 |

## Verdict

PASS with two low findings, neither blocking. `choice_visibility` reuses the module's `single`/`references`/`malformed` helpers, so its refusal shape and members match `channel_endpoints` and `await_anchor`. Checking the operator first is a deliberate, correct difference from the siblings: a `repeat` also carries `owner` and `visible`, so without the check a `repeat` would be read as a choice. The struct exposes only `choice`, `owner` and `visible` (a `Vec` of references in carried order), with no FR-227 record. No panics on caller input, no new dependencies, no wire changes. Gates on 7020b291b: `cargo fmt --check -p qsl-package` clean; `cargo clippy -p qsl-package --all-targets -- -D warnings` clean; `cargo test -p qsl-package --lib protocol_view` 24 passed, 0 failed, including the new test.
