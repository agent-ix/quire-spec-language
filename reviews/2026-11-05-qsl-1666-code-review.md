---
id: SR-20261105001153
title: "Code review — QSL-1666/1668 reader projection"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; PR #2364 diff against ffe18a8bbe3598334591364fd0e158d1bb9bb567: qsl-package/src/checked_v2.rs; qsl-package/src/emit/tests/protocol_locations.rs; qsl-package/src/reader_protocol_projection.rs; qsl-package/src/wire_form.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md
    type: references
  - target: ix://agent-ix/quire-spec-language/spec/functional/FR-400-separate-checked-terms-from-the-checker.md
    type: references
---

## Summary

One code-review pass over the four changed Rust files for PR #2364, covering
FR-390 reader projection and FR-400 checked-term admission. Two reader defects
were reproduced and fixed in the branch: nested binders reused active slots,
and binder lookup did not read admitted parameter names from their checked
terms.

## Verdict

**PASS** — both findings are fixed in the current branch and the focused
regression passes. The post-#2373 aggregate gates remain pending by operator
direction.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `next_slot` used only the frame high-water mark, which is not grown until after projecting a let body. A nested let therefore reused its active parent's slot and could evaluate with the wrong local value. | qsl-package/src/reader_protocol_projection.rs:1967 |
| FND-002 | medium | `binder_node` searched `declaration.qualified_name`, but admitted parameter nodes carry their declared name and lexical level in the checked parameter term and have no declaration. A valid let or collection query could therefore fault during reader projection. | qsl-package/src/reader_protocol_projection.rs:1983 |

## Dispositions

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | b3673d56c: slot allocation now starts after active locals; the nested-let regression verifies distinct consecutive slots and the grown frame size. |
| FND-002 | fixed | 5cd262771: binder lookup reads parameter name/level from admitted terms, uses the active lexical level, and follows the binder body to identify referenced parameter nodes. The focused nested-let regression passes. |

## Coverage

- Reviewed the full PR diff against the recorded base; no workflow files or
  dependency locks changed. No applicable `AssuranceProfile` was found.
- `cargo fmt --all -- --check` and `git diff --check` passed.
- `cargo test --locked -p qsl-package the_reader_projects_nested_let_binders_into_distinct_slots` passed after the fixes.
- `cargo test --locked -p qsl-eval reader_backed_projection_` passed all six reader parity tests before the final lexical-resolution refinement; the refinement was separately covered by the qsl-package regression.
- A broader `cargo test --locked -p qsl-package reader_` run had two failures: the existing protocol-reference emission fixture reported omitted nodes, and the text-literal-vector test could not find the external QSPEC fixture path. These failures were outside the changed reader tests.
- `quoin matrix --repo . --json` was attempted but remained silent and CPU-bound for 5m39s, then was interrupted. Computed matrix evidence and the full repository gap analysis are unavailable. The optional intent-to-test semantic review was skipped.
- Clippy configurations, workspace check, and `xtask string-edge` remain pending until #2373 lands, per the merge hold.
