---
id: SR-2264001
title: "Code review of QSL-1622 monitor documents from clause runs (quire-spec-language PR 2264)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@33095598b7dfca0162ec35098f8c5e82685345a9; qsl-replay/src/outcome.rs; qsl-replay/src/spine/clause.rs; qsl-replay/src/spine/clause/tests/temporal.rs; qsl-replay/src/spine/native_result/temporal.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-286
    type: reviews
---
# SR-2264001: Code review of QSL-1622 (PR 2264)

## Summary

Ticket: QSL-1622. Head 33095598b, reviewed on a trial merge with origin/main d421482f4 (clean merge). Rust lane (rust-review) folded in. Gates on the trial merge, all with qsl-slot and QSPEC_DIR=/Users/peter/dev/quire-specification: fmt 0; check --locked --workspace --all-targets 0; clippy -D warnings default 0 and --no-default-features 0; arch-lint qualified-core 0; arch-lint api-surface --qsl . --qsl-only 0; qsl-foundation 65 passed 0 failed; qsl-inspect 10/0; qsl-analyze 1388/0; root it exit 0; qsl-replay 1031 passed, 14 failed lib + 4 dispatch_link_compile + doctest = the 19 known (13 layers::equality, frame_occurrence_region, 4 dispatch_link_compile, replay_observed_temporal doctest), no new failure.

Mutation runs in a scratch copy (tests: the three new tests): pending to Incomplete label, drop clause member, undefined label on pending, undefined label on violation, reversed item order, category Pending to Inconclusive, wrong indeterminate reason spelling: each fails at least one test. Survivors: Unsupported to Declined, Refusal to Unsupported, Incomplete to Unsupported all pass (FND-002).

Verified against FR-286 table: labels (tested, refuted, pending, indeterminate, incomplete, unsupported, declined, undefined) match; category only from ClauseRunReport::category(); undefined label only on Category::Undefined with cause insufficient-data; pending and indeterminate carry no terminal record members and carry clause; ClauseRunNotAnItem for InternalFailure and Inconclusive (FR-283 whole-request fault); TemporalUndefinedCause::reason is the single undefined-reason spelling. The coder's reading of the cause column is right where the table is explicit (insufficient-data, undefined-evaluation{where, cause}, unsupported_projection/missing-fairness-premise, limit, indeterminate reason), and unspecified for Violated (FND-003).

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | FR-286-AC-8's clause "the same run through the FR-283 monitor operation gives the same item" and TC-770 step 7's last clause are not implemented or tested: no monitor operation exists (qsl-analyze has none; QSL-597 LC9 owns it, state Specify, unassigned), yet FR-286's Delivery line says AC-8 is delivered by QSL-1622 with no carve-out. | spec/functional/FR-286-serialize-every-outcome-as-one-json-outcome-document.md:215,234 | missing-requirement |
| FND-002 | medium | No test pins the incomplete, unsupported and declined label rows or their causes (limit-reached, missing-fairness-premise, refusal catalog code); swapping the labels of Unsupported, Refusal and Incomplete leaves every test green. | qsl-replay/src/outcome.rs:1014-1022 | correct-requirement-no-evidence |
| FND-003 | medium | A violated clause (Violated) builds an item with result refuted and no cause, and no position; the table says cause "FR-331's cause" and FR-283's record carries position, counterexample, witness. Neither the code nor any test fixes what a Violated item carries. | qsl-replay/src/outcome.rs:1128-1134 | missing-requirement |
| FND-004 | low | The FR-501 indeterminate reason is spelled a second time in outcome.rs (indeterminate_reason) beside native_result/temporal.rs ReasonWire; FR-286 says "as its result report spells it", and only incomplete-input is asserted, so the two spellings can drift. | qsl-replay/src/outcome.rs:1052-1079 | implementation-bug-despite-evidence |
| FND-005 | low | The InsufficientData item keeps only kind insufficient-data and drops position, count and min_count that FR-283's record holds; PendingWindow and PendingNextEvent runs are not exercised. | qsl-replay/src/outcome.rs:1122-1126 | correct-requirement-no-evidence |

## Verdict

Code is sound and gates are green on the trial merge. Not mergeable as is: FND-001 needs a lead/spec decision (below), FND-002 and FND-003 are fixes inside this PR.

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | Spec #2266 (c767e7380) splits FR-286-AC-8 to the producer only and moves the monitor-operation half to FR-286-AC-13 and FR-283-AC-14, delivered by QSL-597 per the Delivery line. |
| FND-002 | fixed | incomplete_unsupported_and_declined_rows_carry_their_labels_and_causes added; the three mutations each fail it. |
| FND-003 | fixed | Violated item carries position, counterexample (loop []) and a witness exactly when the report holds one; spec FR-286 table now says no cause. Mutations drop-position, drop-witness-guard and loop each fail a test. |
| FND-004 | fixed | One spelling: ReasonWire From<&TemporalIndeterminateReason> feeds both the result report and the item; parity test over all six reason shapes; a spelling mutation in ReasonWire fails an existing wire test. |
| FND-005 | fixed | count, min_count, position carried and tested; PendingWindow from a real run and PendingNextEvent synthetic tested. |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-006 | low | FR-286-AC-10 says an item has "no cause member" but every item serializes `"cause": null` (the pre-existing writer shape for all items, proof items included); the tests assert is_null, which cannot tell an absent member from a null one. Spec lane should word the AC as "cause null" or the writer should skip it. | qsl-replay/src/outcome.rs:711 | wrong-requirement |
| FND-007 | low | item_record re-reads its own serialized wire as JSON and matches on the kind strings ("violated", "undefined", "insufficient-data", "indeterminate") instead of matching the typed TemporalVerdict; a renamed wire kind silently drops members. A non-temporal disposition also carries a report witness without a Violated row. | qsl-replay/src/spine/native_result.rs:2379-2415 | implementation-bug-despite-evidence |
