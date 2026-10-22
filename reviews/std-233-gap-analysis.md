---
id: SR-13992
title: "gap analysis of STD-233 new and changed ACs against TC rows (PR #1189, QSpec PR #329)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@e2239ccee1cd807c6f824fb928d90717f6798981; spec/functional/FR-205-define-the-protocol-subject-and-its-state-key.md, spec/functional/FR-206-take-the-protocol-step-kinds-with-folded-structural-moves.md, spec/functional/FR-216-give-every-protocol-step-a-static-footprint.md, spec/functional/FR-221-explore-the-x86-tso-memory-model.md, spec/test-cases/TC-650-protocol-subject-and-state-key.md, spec/test-cases/TC-651-protocol-step-kinds.md, spec/test-cases/TC-661-protocol-step-footprints.md, spec/test-cases/TC-666-x86-tso-memory-model.md; agent-ix/quire-specification@d2b6b37bea68292b7b74ee5208b4916b3598166f; spec/functional/protocol/FR-425-define-protocol-subjects-and-state-keys.md, spec/functional/protocol/FR-426-apply-protocol-steps.md, spec/test-cases/TC-375-protocol-state-key-and-coalescing.md, spec/test-cases/TC-376-two-branch-join-worked-example.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-205
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-206
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-216
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-221
    type: reviews
---
# SR-13992: gap analysis of STD-233 (PR #1189, QSpec PR #329)

## Summary

Ticket: STD-233. Plan completion: not assessed. This is a manual mapping of
each new or changed AC to its TC rows. It covers spec only: the code for
these ACs is in the code lane's remaining work, so no new test exists yet.
Mapping:

- FR-205-AC-8 maps to TC-650: Scope is now AC-1 to AC-8, plus step 8 and its
  expected result.
- FR-206-AC-10 maps to TC-651: Scope is now AC-1 to AC-10, plus step 10.
- FR-216-AC-4 (changed) maps to TC-661 step 4, which is not updated
  (FND-001).
- FR-221-AC-1 (changed) maps to TC-666 step 1, which is updated.
- QSpec FR-425-AC-12 maps to TC-375 PK-16.
- QSpec FR-426-AC-16 maps to TC-376 FL-10.

Two existing tests already exercise new ACs and carry no trace tag:
`a_compensation_thread_keys_its_registration` (FR-205-AC-8, registration
part) and `an_effect_waits_for_its_attempts_binder` (FR-206-AC-10), both in
qsl-analyze/tests/it/protocol_compensation.rs.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-216-AC-4's oracle changed, but TC-661 still reads "perturb a location outside its `R` and diff the post-states" with the expected result "every change confined to `W`". The new parts are missing from the TC: equal enabledness from `q'` and `q`, successors that differ only at `l`, and locations outside `W(s)` that keep their pre-state value in both runs. A test written from TC-661 would not check what the AC now asserts. Update TC-661 step 4 and its expected result to the new oracle. | spec/test-cases/TC-661-protocol-step-footprints.md:29-31; spec/test-cases/TC-661-protocol-step-footprints.md:46-47 |
| FND-002 | low | `a_compensation_thread_keys_its_registration` and `an_effect_waits_for_its_attempts_binder` already assert the registration part of FR-205-AC-8 and the behaviour of FR-206-AC-10, but carry no `#[trace]` tag, so the matrix will show those ACs as untested. Tag them `#[trace("TC-650", "FR-205-AC-8")]` and `#[trace("TC-651", "FR-206-AC-10")]` when the code lane lands the remaining parts. | qsl-analyze/tests/it/protocol_compensation.rs:990; qsl-analyze/tests/it/protocol_compensation.rs:1013 |

## Verdict

Every new AC has a TC row in its own repo. The one changed AC whose TC row
was not updated is FR-216-AC-4 (FND-001). The fixture defects in TC-650 step
8 and PK-16 are recorded in SR-13990 FND-007.
