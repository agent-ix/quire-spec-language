---
id: SR-15111
title: "EARS review of STD-258 in-process provider entry over the checked package (PR #1206, QSpec PR #357)"
type: SpecReview
analysis: ears-conformance
scope: "agent-ix/quire-spec-language@48fc9a675a111d90eb03746556d435d7e5eaaa0b; spec/functional/FR-280-serve-the-driver-s-lower-generate-and-prove-operations.md; agent-ix/quire-specification@203fa2efdba64c0fe18dacbf0659d49420439e2d; spec/functional/tooling/FR-300-expose-rust-lifecycle-api.md, spec/objects/interfaces/FR-331-backend-provider-envelope.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-280
    type: reviews
---
# SR-15111: EARS review of STD-258 (PR #1206, QSpec PR #357)

## Summary

Ticket: STD-258. Examined: the new and changed FR-280 Behavior statements
(the `analyze`-over-`CheckedPackage` event, the checked-package ubiquitous
statement, the EN-4 witness statement, the record-equality statement), the
FR-331 "In-process providers" SHALL statements, and the new FR-300
paragraph. The QSL gate's grammar lint reports nothing on FR-280, and the
QSpec gate reports no new diagnostic. The event statement and the
record-equality statement conform. The FR-300 paragraph is descriptive prose,
as the rest of that file is.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | "When an EN-4 item settles `measured` through the provider entry, the sampled witnesses its record keeps shall be the ones `analyze` keeps ..., and each shall replay through `replay_sampled_witness` ..." has two `shall`s, and its subject is the witnesses, not the system. Fix: split it into two statements with the provider entry as subject. | spec/functional/FR-280-serve-the-driver-s-lower-generate-and-prove-operations.md:100-103 |
| FND-002 | low | "its result SHALL not be reused for another request" is passive and names no system: it does not say whether the producer, the driver or the result cache does not reuse it. Fix: name the subject (for example "the producer SHALL NOT reuse ..."). | quire-specification spec/objects/interfaces/FR-331-backend-provider-envelope.md:342-343 |

## Verdict

Mergeable for EARS. Two low findings, no major ones.

## Disposition pass 1

Disposition pass 1 at QSL 203b8c172 and QSpec 4248c9f08 (fix commits 203b8c172 and 4248c9f08, diffs only). Gates in detached worktrees: QSL `make check-spec-validation` exits 0 with no diagnostic on FR-280, TC-761 or ADR-029; QSpec `quire validate --scope . 'spec/**/*.md'` gives 86 fail lines (FR-331's three diagnostics are the ones on main); `cargo test --test checked_package_v2 --test backend_provider_v1` passes 126 and 23. QSL PR #1206 is CONFLICTING with main: FR-280-AC-4 (main renamed `BackendDescriptor::admit` to `descriptor_from_manifest`, #1204) beside the edited AC-5 row.

Disposition pass 1: both low findings fixed; no new EARS finding in the fix diffs.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | The statement is split: "the provider entry shall keep in its record the sampled witnesses `analyze` keeps for the same item" and "The provider entry shall keep each sampled witness replayable through `replay_sampled_witness` over the checked subject it was given (FR-193)". |
| FND-002 | fixed | "the producer SHALL NOT reuse the result of one in-process request for another, and SHALL run the engine once for each in-process request". |
