---
id: SR-1280
title: "Spec review of quire-spec-language LC1: status rows and the FR-278 composition wording"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@ddd162c7bf269ee9dfe4b8e3aec50341993c256b; git diff origin/main...HEAD -- spec/ (PR #624): FR-278 statement and AC-1/AC-2 edits, FR-027/FR-100/other spine::compile rewordings, spec/spec.md FR-275/FR-276/FR-278/FR-285 rows, spec/tests.md TC-755..759 and TC-769 rows"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-278
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-275
    type: reviews
---
# Spec review of quire-spec-language LC1

## Summary

Ticket: QSL-489 (LC1), PR #624. The spec edits are rewordings that replace
`qsl_replay::spine::compile` with "the FR-278 composition", plus status rows.

The FR-278 edit matches the ruled design: the monolith is deleted with no
wrapper, and each caller composes the four operations. Its new AC-1 and AC-2
oracle, "the bytes FR-027's `compile` command writes", is a real external
oracle. It is better than comparing against a function that no longer exists.

The rewordings in FR-027, FR-098, FR-099, FR-100, FR-110 and the TC files are
consistent.

## Verdict

Changes requested, on the status rows only.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The rewritten rows list the remaining work with no owner, so the work has no home. Specifically: FR-275's "remaining" list, FR-276's "the operations that do not exist yet" and its FR-276-AC-4, TC-756 (monitor and replay), and TC-757 step 4. FR-285's and TC-769's `StageFailure::Fault` used to say "owned by slice LC1", and this PR dropped that. Fix: name the owner on each row. `StageFailure::Fault` and FR-275-AC-5 stay with LC1 (SR-1278 FND-002, SR-1279 FND-003). The other operations go to the FR-003, FR-281, FR-283, FR-297, FR-298, FR-355 and FR-098 slices, and FR-277-AC-2 to the ADR-030 depth work (QSL-381 series). | spec/spec.md FR-275, FR-276, FR-285 rows; spec/tests.md TC-756, TC-757, TC-769 rows |
| FND-002 | low | The FR-275 row says "`parse`, `select`, `check`, `package` and `execute` (`qsl_replay::spine`) take their predecessor's own type and a caller-owned `&Cancel`". That reads as if `execute` were a public spine operation, but it is crate-internal (FR-100-AC-8). Fix: say that `execute` is crate-internal until FR-279. | spec/spec.md FR-275 row |
