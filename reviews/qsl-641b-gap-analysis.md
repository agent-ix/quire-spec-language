---
id: SR-1354
title: "Gap analysis of quire-spec-language PR #650: report claim identity and operator-parity obligation recompute (QSL-641)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@4c2cf5dab9e9f1629d188b9f72001435b4a2d793; PR #650 diff against origin/main; FR-357-AC-13..AC-16 (new), FR-357-AC-1..AC-11 (re-checked after the API change); trace tags TC-904 in qsl-replay/src/execute/tests.rs, TC-177/TC-178 in qsl-replay/src/proof_result.rs, TC-770 in qsl-replay/src/outcome.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-357
    type: reviews
---
# Gap analysis of quire-spec-language PR #650

## Summary

Ticket: QSL-641 (follow-up). Planless. Plan completion: not assessed.

`quoin matrix --json` (quoin 0.28.1) at this head lists FR-357-AC-1 to
FR-357-AC-16 as `tagged`. The new criteria are bound as follows:

- FR-357-AC-13: `tc_904_a_value_report_carries_the_sent_claim_on_every_outcome`
  and `operator_arm::tc_904_an_operator_report_carries_the_sent_claim_on_every_outcome`.
- FR-357-AC-14: `operator_arm::tc_904_the_observation_digest_is_carried_unchanged_and_never_recomputed`.
- FR-357-AC-15: `operator_arm::tc_904_the_obligation_identity_is_recomputed_from_the_o09_preimage`.
- FR-357-AC-16: `operator_arm::tc_904_a_tampered_obligation_identity_is_refused`.

Every binding is correct. The AC-15 test is independent of the encoder
under test. It hashes canonical text written out by hand with
`qsl_foundation::ByteDigest::of`, a plain SHA-256, and never calls
`quire-canonical`. AC-1 to AC-11 still hold after the refactor: every
existing test now reads `report.result()`, and the FR-098 refusals moved
from `Err` to `ValueParityResult::Refused` with the same causes asserted.

No code is left without an owning requirement. `operator_obligation.rs`
traces to FR-357's recompute bullet and ADR-013 O-09. No stubs and no
coverage inflation were found.

The missing package check (preimage members against the recompiled
package) is recorded once, as SR-1353 FND-001. The weak outcome assertions
in the AC-13 and AC-14 operator tests are SR-1353 FND-003. Neither is
repeated here.

## Verdict

Approve, with the one low gap below. The four new criteria are tagged, and
their tests assert what each criterion states. The code-side gaps are in
SR-1353.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The AC-15 hand-written canonical text covers only non-negative ranges (`"0"`, `"100"`, `"7"`). ADR-013 O-09 writes range bounds as decimal strings, and operand ranges can be negative, but no test pins the form of a negative bound (for example `"-5"`) or of `i64::MIN`. An encoder that wrote a negative bound some other way (as a JSON number, or with a leading `+`) would still pass. Fix: add one operand range with a negative lower bound to the hand-written AC-15 text. | qsl-replay/src/execute/tests.rs:2396-2462 |

## Dispositions

Round 1, reviewed at 0679c3acb1a4315d8bf15c850a9d44fbcdab799f (fix commits 4c2cf5da..0679c3ac).

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 56b87ba1a |
