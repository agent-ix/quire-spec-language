---
id: SR-20261103161224
title: "QSL-1568 r1 spec review of PR #2204 (FR-501 usable order, AC-26, TC-1295 row 40)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language; PR #2204 head bb13ab56e; spec/functional/FR-501-evaluate-a-temporal-clause-under-the-timestamped-event-finite-window-profile.md; spec/test-cases/TC-1295-evaluate-the-finite-window-profile-on-the-spine.md; context FR-330, FR-522, FR-525, QSpec FR-090"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-501
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-525
    type: references
---

## Summary

Ticket: QSL-1568. Reviewed the semantic ruling in PR #2204 at `bb13ab56e`. Two observations at one stamp whose order keys come from one authority and are equal have no usable order. An order-sensitive operator (`until`, `release`, `since`, `triggered`) reading the pair refuses at the later position with the existing `UnorderedTie` (`invalid_runtime_input`/`invalid-value`, stage `evaluate`). Observation identity never breaks the tie. Examined: the FR-501 refusal bullet, the new "usable order" definition, the Canonical order section, FR-501-AC-7, AC-20, AC-21 and the new AC-26, TC-1295 rows 17, 18, 34, 35 and 40, the FR-525 legacy map rows for FR-043-AC-5 and FR-052-AC-14, FR-522 "Result identity", FR-330, and QSpec FR-090 (the tie statement and FR-090-AC-4). `make check-spec-validation` passes at the PR head with `QSPEC_DIR` set to a fresh QSpec worktree of `origin/main`. It prints only warnings for files this PR does not touch.

## Verdict

**APPROVE WITH CHANGES (two small spec edits).** The ruling is right. QSpec FR-090 says the evaluator shall not "break a tie for an order-sensitive operator" without an admitted order. FR-090-AC-4 refuses equal timestamps "without an admitted causal/sequence order". A key that one authority repeats at one stamp puts both observations at the same point of that authority's sequence, so it admits no order between them. Observation identity is a content identity (FR-106), not a causal or sequence order, so the identity tie-break of PR #2169 substituted a non-admitted order. That is the same kind of substitution FR-090 forbids for ingestion order.

Two alternatives were considered and rejected:

- **`Indeterminate` or `Pending`.** These are for data that progress or completeness could still supply. No later observation can create an order between two already-admitted observations, so the deficit is in the input itself. That makes the refusal consistent with the keyless and cross-authority cases in AC-7.
- **Refusing a repeated key at `admit` for every clause.** FR-090-AC-4 scopes the refusal to the four order-sensitive operators. AC-7 already lets `eventually` evaluate over an unordered tie, so an admit-time refusal would be stricter than both.

The refusal stays scoped:

- `eventually` and `always` still evaluate. AC-26 states this and the test checks it.
- Distinct keys of one authority still order, as AC-7 and AC-20 require.
- Cross-authority and keyless pairs already refused, and still do.

The Canonical order bullet explains that the identity step only fixes indices, so the outcome and the `result_identity` do not depend on listing order. That keeps FR-522's identity rule and AC-21 intact. FR-330 has no order-key clause that conflicts. Two defects remain:

- **FND-001.** AC-7 and TC-1295 row 18 still say that keys "from one order authority" make `until` evaluate, which is now true only for distinct keys.
- **FND-002.** The FR-525 successor range for FR-052-AC-14 stops at AC-25.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-501-AC-7 still reads "With keys from one order authority on positions 1 and 2 `until` evaluates", and TC-1295 row 18 reads "order keys, one authority ... evaluates". Under the new usable-order definition this holds only when the two keys differ. Read literally with two equal keys of one authority, AC-7 now contradicts AC-26. AC-20 ("the `until` of AC-7 over keys from one authority") inherits the same wording. Say "distinct keys from one order authority" in AC-7 and row 18 (AC-20 then follows). | spec/functional/FR-501-evaluate-a-temporal-clause-under-the-timestamped-event-finite-window-profile.md:348,361; spec/test-cases/TC-1295-evaluate-the-finite-window-profile-on-the-spine.md:56 |
| FND-002 | low | The FR-525 legacy map row for FR-052-AC-14 names "FR-501-AC-1 to AC-25". FR-052-AC-14 says the obligation "is evaluated as FR-501 states", so its successor is the whole of FR-501, and the range now omits AC-26. Name FR-501 as a whole, or extend the range to AC-26, so that the next AC does not cause the same drift. TC-1295's description was extended to AC-26 in this PR but FR-525 was not. | spec/functional/FR-525-map-each-legacy-temporal-evaluator-criterion-to-its-spine-successor.md:125 |
