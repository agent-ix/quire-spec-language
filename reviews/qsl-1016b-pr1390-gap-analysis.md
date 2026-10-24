---
id: SR-16311
title: "Gap analysis of QSL-1016b value-parity profile selections fixed empty (QSL PR #1390)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@3aa38f6ce311416b06a3ce0d0622092515769987; FR-484-AC-9, FR-484-AC-11..13 and FR-484 Behavior (STD-443) against qsl-replay/src/execute/tests/value_parity_identity.rs and tests/it/value_parity_profile_selections.rs (TC-1252 VP-11..13), and qsl-replay/src/execute/value_parity.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-484
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-1252
    type: reviews
---
# SR-16311: Gap analysis of QSL-1016b value-parity profile selections fixed empty (QSL PR #1390)

## Summary

Ticket: QSL-1016 (PR #1390). Planless gap analysis over `git diff origin/qsl-1016-impl...3aa38f6ce`.
Plan completion: not assessed. Each of FR-484-AC-11, AC-12 and AC-13 has one
`#[trace("TC-1252", "FR-484-AC-N")]` test, and all three ran and passed. AC-9's `[]` packet is
covered by #1386's test. For each test I checked which source change would make it fail, and ran
two mutations:

- AC-12 (`tc_1252_a_profile_selection_refuses_before_the_recompile`). The source bytes are
  replaced by a consistent but uncompilable source, so a recompile before the check changes the
  refusal. Mutation: moving the check after `recompile` made this test FAIL (line 699). The test
  asserts the variant, the one selection, the exact display text, code, cause, the terminal value
  and `claim() == ValueIdentity::sent`. It runs over generated 4 and 5, the all-zero identity and
  a stale `package_id`. The unknown profile and both mapped lock rows refuse at decode
  `unknown_profile`/`unsupported-selection`. That decode case also fails if the check moved ahead
  of decode onto the wire.
- AC-13 (`tc_1252_the_target_refusal_precedes_the_profile_refusal`). Mutation: moving the check
  before `function_target` made this test FAIL. Both `frame_operation` and `state_clause` are
  covered through `non_function_targets`, with the uncompilable source.
- AC-11 (`tests/it/value_parity_profile_selections.rs`). `UC` matches TC-1252's text. The lock is
  read from `call_site`'s package bytes and has exactly the two rows. The packet's `[]` admits.
  The request is built from the envelope and decodes. It replays `Agrees` (4) and `Diverged` (5),
  and each `claim()` holds `UC`'s `package_id`. The test fails if the replay ever required the
  request to mirror the lock rows.
- Recorded gap (the lock lists `protocol_profile` before `temporal_profile`; the test sorts by
  role): acceptable. AC-11 says "holds exactly the two rows" and states no order, so sorting both
  sides keeps "exactly these two rows" and does not pin an unspecified order. The emitted order is
  also the role order, so nothing is hidden.

Reverse gap: the new code (`UnusedProfileSelections` and the check in `settle`) is owned by FR-484
Behavior. There are no stubs, no ignored tests and no tautological assertions.

## Verdict

**PASS**: one low trace/coverage finding, which does not block merge.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | FR-484-AC-11's last clause (AC-4's unit has lock `profile_selections` `[]` and the same request `[]`) is asserted only inside the AC-9-tagged test via `assert_lock_selects_no_profile`, whose doc comment still says "pending FR-484 profile-selection ruling" although STD-443 is that ruling and this PR implements it; the AC-11-tagged test does not cover the clause | qsl-replay/src/execute/tests/value_parity_identity.rs:499 |

## Coverage

- Criteria examined: FR-484-AC-9, AC-11, AC-12 and AC-13, and FR-484 Behavior's
  profile-selection rule. All four are tagged, and all ran and passed at 3aa38f6ce.
- Matrix method: grep fallback over `#[trace("TC-1252", ...)]`. `quire matrix` was not run while
  gates are running on the machine.
- Semantic review (intent to test to code): performed for FR-484 AC-11..13.
- Plan completion: not assessed
