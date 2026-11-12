---
id: SR-2026111204015000000002
title: "gap analysis of PR 2684 FR-519 choice_visibility accessor"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; spec/functional/FR-519-publish-the-protocol-read-surface-the-quire-protocol-link-layer-imports.md, spec/test-cases/TC-1314-publish-the-protocol-read-surface-the-quire-protocol-link-layer-imports.md, qsl-package/src/protocol_view.rs, qsl-package/src/emit/tests/protocol_view.rs"
review_set: subset
---

## Summary

Ticket: QSL-1890. Planless gap analysis of PR 2684 at 7020b291b. Plan completion: not assessed. `quoin matrix --json` shows FR-519-AC-20 `static_status: tagged`, bound to `a_choices_owner_and_visible_set_are_the_nodes_it_references`. The ticket's asks are all present: an FR-519 AC in the same PR as the code, the public accessor, MissingMember/MalformedMember refusals like the sibling views, and a tagged test. Checked against QSpec FR-172 "Choice visibility carrier", FR-172-AC-6 and the FR-466 `choice` layout row.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The AC-20 clause "`ChoiceVisibility` ... carries no FR-227 visibility record" is backed only by `let _: ChoiceVisibility<'_> = found;`, a type ascription that would still compile if a field were added. An exhaustive destructure such as `let ChoiceVisibility { choice: _, owner: _, visible: _ } = found;` would make the test fail to compile if the struct gained a member. | qsl-package/src/emit/tests/protocol_view.rs:1867 |
| FND-002 | low | AC-20 and the doc comment say the visible entries are `expression` nodes (QSpec node tag `expression`). The test asserts only `semantic_form == "query"` and never `node_tag == "expression"`, so the tag half of the claim is unasserted. | qsl-package/src/emit/tests/protocol_view.rs:1864-1866 |

## Verdict

PASS with two low test-intent findings. Every other AC-20 clause has an assertion: the id, owner and visible references equal the in-memory node, the owner is a `role`, and there are the MissingMember/MalformedMember refusals for `owner` and `visible` plus the operator refusal over `Poll`. Binding of the test to FR-519-AC-20 / TC-1314 is correct. FR-172-AC-6 (no full FR-227 record) is met by the struct's three fields. `visible` as a `Vec` of node references matches FR-466's "a set of the `expression` nodes".
