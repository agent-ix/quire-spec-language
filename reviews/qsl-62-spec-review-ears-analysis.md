---
id: SR-1194
title: "EARS analysis of quire-spec-language PR #588 commit 6ffdca6f (QSL-62): FR-355 statements and acceptance criteria"
type: SpecReview
analysis: ears-conformance
scope: "agent-ix/quire-spec-language@6ffdca6fe18c174773ef1b36e335bcc4cd7a33f2; PR #588 commit 6ffdca6f against main 2ece5712"
review_set: subset
---
# EARS analysis of quire-spec-language PR #588 commit 6ffdca6f (QSL-62)

## Summary

Ticket: QSL-62. Checked every SHALL statement in FR-355's Description and Behavior against the EARS patterns, and each AC for testability. Every statement names `check_fences` as its subject and is ubiquitous or unwanted-behaviour form ("When an artifact holds a `quire` fence and declares no object type ..."). Each is atomic, apart from the source-map bullet, which joins two linked obligations (locate through quire-rs, verify the map first) that are tested together. `quire validate` on FR-355, TC-901 and US-036 reports no grammar warning.

Examined:
- FR-355 Behavior (examined)
- FR-355 Behavior (missing type) (examined)
- FR-355-AC-2 (examined)
- FR-355-AC-5 (examined)

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | AC-5's second half tests cancellation, but FR-355's Behavior has no cancellation statement. The Inputs list `cancel` and the Outputs' StageFailure list ("FR-056's intake refusals, an extraction that is not available, and a reached limit") omits `Cancelled`. Add a Behavior bullet ("When `cancel` is set, `check_fences` SHALL stop within one charge and return `StageFailure::Cancelled` with no report", per ADR-029 LC-3) and add `Cancelled` to the Outputs list. | spec/functional/FR-355-check-the-quire-fences-of-spec-artifacts.md:97 |
| FND-002 | low | AC-2 gives the other two refusals an exact code (`ill_typed`/`type-mismatch`, `missing_declaration`/`missing-name`) but says only "a parse refusal" for `self.suspended and`, and TC-901 step 2 repeats that. A test cannot check which refusal it is. Name the S1 code and cause the incomplete binary expression gives. | spec/functional/FR-355-check-the-quire-fences-of-spec-artifacts.md:94 |

## Verdict

Two low findings: AC-5's cancellation result has no Behavior statement, and AC-2's "a parse refusal" gives no code. Mergeable once they are fixed.
