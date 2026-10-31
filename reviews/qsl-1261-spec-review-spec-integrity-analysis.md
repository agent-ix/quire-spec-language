---
id: SR-1926003
title: "spec integrity of PR #1926 (QSL-1261): requirements left naming deleted machinery"
type: SpecReview
analysis: integrity
date: 2026-10-31
scope: "agent-ix/quire-spec-language; PR #1926 spec/functional/FR-273, spec/test-cases/TC-1111, and the requirements the deleted code served (FR-286, NFR-015)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-286
    type: references
  - target: ix://agent-ix/quire-spec-language/NFR-015
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-273
    type: references
---
# SR-1926003: spec integrity of PR #1926 (QSL-1261)

## Summary

Ticket: QSL-1261. Checked that the spec edits are consistent with the deletions, and searched `spec/` for every requirement that still names the deleted check or artifact identity.

Examined, clean: FR-273 row 21 and FR-273-AC-21 (withdrawn in place, id kept); TC-1111 step 6 and expected result 6. No FR names `transcript_digest`, `ByteDigest` or `DigestRecord` (ADR-010 DA-15 is an observed baseline, left as history).

Note, not a finding (pre-existing on main, not introduced here): FR-286's description and FR-286-AC-1/AC-6 still say `artifacts` carries the `package_id`, while main's TC-770 test already asserts `"artifacts": []` for the `package` outcome and `with_artifacts` had no caller. This PR makes `[]` permanent. FR-286 should be amended to say `artifacts` is always empty, in this PR or a follow-up.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | NFR-015-AC-7 still requires "the `arch-lint canonical-encoder` check passes". The PR deletes that check and drops it from TC-1111 step 6, so the AC names a check that no longer exists and its verifying TC no longer checks it. Fix: drop the clause from NFR-015-AC-7. | spec/non-functional/NFR-015-build-evaluator-crates-no-std-behind-a-default-std-feature.md:217 | wrong-requirement |

## Verdict

**Not mergeable until FND-001 is fixed** (a one-clause spec edit in this PR). The code is correct.
