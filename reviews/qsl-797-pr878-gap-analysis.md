---
id: SR-11410
title: "Gap analysis of quire-spec-language PR #878: FrameChange::Link wire and replay decode"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@b35563aa0d7a42adf308f5eaf3156d3829ed19b0; qsl-replay/src/spine/clause/tests/frame.rs, qsl-replay/src/witness/frame.rs, qsl-replay/src/spine/native_result.rs, spec/functional/FR-115-run-an-operation-frame-over-an-invocation.md, spec/functional/FR-267-write-run-results-as-native-run-result-2.md, spec/test-cases/TC-1037-a-frame-run-reports-a-changed-relationship-link-as-a-link-witness.md"
review_set: subset
---
# Gap analysis of quire-spec-language PR #878

## Summary

Ticket: QSL-797 (implementation slice a). Planless gap analysis over the PR
diff: which acceptance criteria the new tests claim, and whether each tagged
test asserts that criterion. Plan completion: not assessed.

Examined: FR-115-AC-8, FR-267-AC-12, FR-116-AC-7, TC-1037 rows 2, 13, 14, 15,
QSpec TC-255 NR-11 (by id), and the PR body's done and not-done lists.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | `a_link_finding_routes_to_a_violation_with_exit_ten` is tagged `FR-115-AC-8`, but it sets `report.disposition = ClauseDisposition::FrameViolation(link_witness(..))` by hand and then reads `stage()`, `category()`, `truth()` and `exit_code()`. Those methods match `FrameViolation(_)` and never look at the change, so no Link code is exercised and the test cannot fail for any defect in AC-8's behaviour. AC-8 requires a Frame run over invocation I to report success when `rehome` modifies `Pump-housing-Tank`, a violation with the `Link` witness when `modifies` is empty, the `p3`-`t1` `added` case and a retained field-only `modifies`. That needs the step 5 producer, which is out of scope (QSL-794). The tag makes the matrix count AC-8 as tested when none of it is. Fix: drop `FR-115-AC-8` from this test's `#[trace]` (keep it untagged or delete it), and tag AC-8 in QSL-794 when the producer lands. | qsl-replay/src/spine/clause/tests/frame.rs:486-500 |

## Verdict

FAIL, one blocking finding (FND-001): an overclaiming trace tag.

Checked and correct:
- `native_result_two_round_trips_a_link_change` (FR-267-AC-12, row 13) writes
  the report, compares the `change` with a JSON literal from the spec, checks
  `violation`, `truth: false`, `basis: closed-scope`, no outcome, no top-level
  `witness` and `population` `Q`, and compares the read-back witness with the
  input witness, which is not the same value. The binding is correct.
- `native_result_two_refuses_broken_link_changes` (FR-267-AC-12, row 14) covers
  every row 14 and NR-11 edit: each link member removed, `population` removed
  from `source`, `key` removed from `target`, an unknown member, `difference`
  `moved` and `null`, a bare-key `source`, and the extra population refusal.
  Each edit starts from a valid document. The binding is correct.
- The `each_frame_change_decodes_to_its_claimed_change` Link case backs the
  FR-116 decode. It carries no new tag, so it does not claim FR-116-AC-7, and
  row 15 settlement is honestly deferred.
- The PR body says the producer and TC-1037 rows 1, 3 to 12 and 15 are the next
  slice (QSL-794). No tag claims them. The body's "Ids" line lists
  FR-115-AC-8..10 and row 2 as related ids. Only the FR-115-AC-8 tag (FND-001)
  claims more than the code shows.

Recorded (not blocking):
- low: all three tests carry the bare `TC-1037` id although they cover only
  rows 13 and 14 of 15. That is how the repo tags tests (`TC-514` the same way).
