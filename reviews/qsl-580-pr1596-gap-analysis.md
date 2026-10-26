---
id: SR-1596002
title: "gap analysis of PR #1596 (QSL-580b, FR-231 timed subject)"
type: SpecReview
analysis: gap-analysis
date: 2026-10-26
scope: "agent-ix/quire-spec-language; PR #1596 own commit 48f73e4ff..ddeff5dd6; spec/functional/FR-231 AC-1 to AC-4, FR-230-AC-4; qsl-eval/tests/it/timed_subject.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-231
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-230
    type: reviews
---
# SR-1596002: gap analysis of PR #1596 (QSL-580b, FR-231 timed subject)

## Summary

Ticket: QSL-580. Plan completion: not assessed. Planless audit of the PR's tags and code. FR-231-AC-1 and AC-3 are fully asserted. FR-231-AC-4 is tagged, but only its first half is asserted.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `a_claim_reads_alike_with_and_without_time_stamps` carries FR-231-AC-4, but the AC's second half ("a claim atom `c.x <= 3 ms` reads the position's clock valuation") is unasserted. The PR body says it is not built yet: `BehaviourLetters` reads only the discrete state. Drop the tag until it is, or split the AC. | qsl-eval/tests/it/timed_subject.rs:602; qsl-eval/src/simulation/model/timed.rs:930-977 |
| FND-002 | low | The AC-2 half "under event-position false-extension the execution closes at position 2" is asserted with `timed_trace(false)` and a stamp count of 3. No unit selects the event-position profile, so the profile reading itself is unchecked. | qsl-eval/tests/it/timed_subject.rs:463-496 |

## Verdict

**CONDITIONAL.** No untagged claimed AC, no stub, no unowned code. Fix the FR-231-AC-4 partial tag (FND-001).

## Coverage

Plan completion: not assessed. Semantic review: done by hand for the claimed ACs.
