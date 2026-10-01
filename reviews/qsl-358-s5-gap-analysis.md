---
id: SR-966
title: "QSL-358 slice 5 gap analysis of PR 570: moved call-surface vocabulary against FR-068 AC-4/CON-3 and the slice's own claims"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@0ff6644bdafd4b07469b86b08d993c1fd4bfa582; diff 0a399675...0ff6644b; xtask/src/definition_scan.rs; quire-semantic-value/src/*.rs; qsl-eval/src/value/expression/mod.rs; spec/functional/FR-068-*.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-068
    type: reviews
---
## Summary

Ticket: QSL-358 (slice 5). The slice's claims were mapped to the tests that back
them, at 0ff6644b:

- CON-3 symbols defined once in SV (Location, Origin, CheckingLimits,
  DepthAboveMaximum, MAX_CHECKING_DEPTH, CheckMode):
  `con3_methods_and_symbols_each_have_exactly_one_defining_location`, TC-170.
  This holds, within the scope limit in SR-963 FND-001.
- AC-4 split (Location, Origin, InputRefusal in SV):
  `twelve_check_cause_types_are_defined_exactly_once_under_check` and
  `input_refusal_is_defined_exactly_once_in_semantic_value`, TC-173. Holds.
- `input_refusal_code` parity: a test in qsl-eval value/expression/mod.rs pins
  all six variants. Holds.
- `cause()` tags: an SV call.rs test pins all six. Holds.
- CheckingLimits depth refusal and builders: SV checking.rs tests. Holds.
- `Location::child`: SV location.rs test. Holds.
- Behaviour unchanged: the existing qsl-eval, qsl-replay and root goldens pass
  at 0ff6644b in `make ci`, with no golden files touched.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The PR says the vocabulary is moved with no duplicates, but ValueLoss, LocatedLoss and the three DEFAULT_CHECKING_* constants are in no definition-scan assertion. Measured: appending `pub enum ValueLoss {}` to qsl-eval evaluate.rs passes every definition_scan test. Only the compiler catches it, through an E0255 clash with that file's import. A copy in any other file of qsl-eval compiles and passes. Add the five names to the SV once-only loop (with SR-963 FND-001's unfiltered count). | xtask/src/definition_scan.rs:461-472 |

## Verdict

One low gap. Every claim of the slice has a test with a real oracle, except
that ValueLoss, LocatedLoss and the DEFAULT_CHECKING_* constants are not
protected against duplicates. The bindings are correct against AC-4's amended
text. The AC's own self-contradiction is in SR-965.

## Dispositions

Round 1 at 553729e5 (rebased onto main 09eb9dc6, which includes slices 2 and 3). The fix commit is 49ae8c6c. `git range-diff 0a399675..0ff6644b 09eb9dc6..a97741d9` shows the seven reviewed commits carried over with only rebase import and Cargo-comment changes. a97741d9 adds import fixups for slice 2. Ids remapped by the coder because of a collision with #562: SR-963 -> SR-965, SR-964 -> SR-966, SR-965 -> SR-967. Cross-references inside the original findings text keep the old ids. Process note: the coder wrote a `## Dispositions` section into each committed copy under `reviews/` (eb7c147b, 553729e5). Only the reviewer records dispositions. The tables below replace those sections, and the next commit of these files must take this copy.

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 49ae8c6c: `ValueLoss` and `LocatedLoss` are in `input_refusal_and_losses_are_defined_exactly_once_in_semantic_value`, and the three `DEFAULT_CHECKING_*` constants are in the CON-3 SV loop, all counted across every scanned tree. Planted copies of ValueLoss, LocatedLoss and DEFAULT_CHECKING_NODES each fail (measured at 553729e5). |
