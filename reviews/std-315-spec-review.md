---
id: SR-16140
title: "Spec review — bind_abstraction_at clears the remembered emission (NFR-015)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language; spec/non-functional/NFR-015-build-evaluator-crates-no-std-behind-a-default-std-feature.md, spec/test-cases/TC-1111-qsl-package-gates-the-checked-package-wire-behind-std.md at 3c24ef7dd"
review_set: subset
evaluated_revision: "3c24ef7dd185606a103aeb246c8c7b36409803b2"
relationships:
  - target: ix://agent-ix/quire-spec-language/NFR-015
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-1111
    type: reviews
---

## Summary

Ticket: STD-315. PR quire-spec-language#1276 states that both `bind_abstraction`
and `bind_abstraction_at` clear the remembered emission on a successful bind, that a refused
binding leaves it unchanged, adds NFR-015-AC-10 and TC-1111 step 8. Reviewed
`git diff origin/main...HEAD` at 3c24ef7dd against `qsl-package/src/checked.rs` on main
(`bind_abstraction` resets `emission`; `bind_abstraction_at` does not; both document "a
refusal leaves the package unchanged") and #1233's NFR-015 hunks.

The rationale (the bound relation and, for `bind_abstraction_at`, one `declaration`
occurrence per site change the emitted bytes, QSpec FR-451) matches the code's doc comment.
NFR-015-AC-10 is unique across main and all 24 open PR heads. #1233 edits the per-crate
table, the Gate paragraph and the AC-8 row; `git merge-tree` of #1276 with #1233, and with
every other open QSL PR, is clean, as the PR body says. `make check-spec-validation` exits 0.
"no longer identifies" is rationale, not changelog; wording is behaviour-only.

## Verdict

MERGEABLE: one low wording finding. The code follow-up the PR body names
(`bind_abstraction_at` resetting `emission` on success) is needed for AC-10 to pass and
belongs to the implementing ticket.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | "`bind_abstraction_at` with bindings it refuses" names no refused binding, and TC-1111 step 8 does not either, so two implementers may pick refusals that exercise different paths (for example one rejected before `check_abstraction` runs). Name one, such as a binding to a declaration the package does not hold. | spec/non-functional/NFR-015-build-evaluator-crates-no-std-behind-a-default-std-feature.md:235; spec/test-cases/TC-1111-qsl-package-gates-the-checked-package-wire-behind-std.md:48 |
