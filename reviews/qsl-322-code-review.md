---
id: SR-813
title: "QSL-322 code review (with rust-review lane) of PR 536"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@b95b671a01f151632be03adb078b83d7280d51ae; qsl-replay/src/witness.rs; qsl-replay/src/execute.rs; qsl-replay/src/execute/tests.rs; qsl-replay/src/identity.rs; qsl-replay/src/lib.rs; qsl-replay/src/request.rs; qsl-replay/src/call_site.rs; qsl-replay/src/witness/frame.rs (unchanged, context)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-070
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-098
    type: reviews
---
## Summary

Ticket: QSL-322. PR: quire-spec-language#536 at b95b671a, base origin/main.
Methods: code-review with the rust-review lane folded in.

The PR replaces `Witness::decode(&[String]) -> Vec<i64>` with
`decode(&[WitnessBinding]) -> Vec<WitnessValue>`, adds the QSL-owned
`WitnessBinding { parameter: WireNodeId, value_type: WitnessValueType }`,
`WitnessValueType { Boolean, I64 }` and `WitnessValue { Boolean(bool),
Integer(i64) }`, widens `CanonicalAssignment.value` to `WitnessValue`, adds
`DecodeRefusal::{Duplicate, Unbound}` and types `Missing`/`Malformed` by
`WireNodeId`, and routes both executor arms through a typed `argument` that
refuses `WrongValueKind` on a Boolean/Integer mismatch.

Checks run:

1. Decode join. `decode` joins each entry to a binding by
   `WireNodeId::to_string()` equality, never by position, and returns values
   in binding order. It refuses `Unbound` first (any entry naming no
   binding), then per binding `Missing`, `Duplicate`, `Malformed`. Correct
   for well-formed entries. See FND-003 for the one entry shape it skips.
2. Typed read. `WitnessValueType::read` is marked `#[string_edge]`;
   `Boolean` admits exactly `0`/`1`, `I64` uses `str::parse::<i64>`, so an
   overflow refuses `Malformed` (tested with `i64::MAX` followed by `0`).
3. Executor. The `Witness` arm builds one binding per parameter from
   `call.types`, refusing `WrongValueKind` for a type no witness value has
   before decoding. The `Input` arm now carries `WitnessValue` and
   `argument` refuses both mismatch directions. The `Witness` arm can never
   hit the mismatch branch, because `decode` already reads each value as its
   binding's type. The match is exhaustive, with no wildcard.
4. Redaction. `ReplaySource::Input`'s `Debug` digest now tags each value
   (`0`+bool / `1`+8 LE bytes), so a Boolean and an Integer with the same
   bit never digest alike. `measured_encoded_bytes` keeps `32 + 8` per entry
   as an upper bound; a Boolean is smaller, so the bound still holds.
5. Callers. No other in-repo crate, and neither CG nor IR main checkouts,
   call `Witness::decode` or build `CanonicalAssignment`. The only in-repo
   transcript that carried an extra entry was the old `x=8` test, which now
   asserts `Unbound`. The Unbound behaviour change breaks no caller.
6. Test oracles. `tc_180` now asserts decoded values, not only
   self-equality. `tc_444_decode_reads_typed_values_by_parameter_node_id`
   lists transcript entries in a different order from the bindings, and
   covers `i64::MIN`/`i64::MAX` and both Boolean bits.
   `tc_444_decode_refuses_each_join_failure_with_its_own_variant` hits every
   variant. Mutations I traced by inspection (not executed; reviewer-only,
   no edits): dropping the Unbound scan, dropping the Duplicate check,
   swapping the Boolean bit map, joining by position, or accepting
   Integer for a Boolean in `argument` each turn at least one test red.
7. rust-review lane. No new `unwrap`/`expect` in production paths, no
   `unsafe`, no `as` casts (`u8::from(bool)`), `thiserror` refusal enum with
   named fields for `Malformed`. The `names: Vec<String>` scan is O(n*m)
   over entries and bindings. Both are bounded by the FR-070 reader bound,
   so this is acceptable.

## Verdict

Code is correct on its main path and the tests are strong. FND-001 is a new
doc claim that contradicts the decided ownership ruling (IR names no QSL
type). FND-003 is a real hole in the new strict-entry rule. FND-002 is a
stale doc comment that the ticket named. All three are small fixes. Gate
results are in the report and the gap-analysis SR (SR-814).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The new `identity` and crate docs say IR names QSL's `ObligationIdentity` directly: identity.rs says "IR and CG name this type directly" and lib.rs says "which IR and CG name directly". The decided ruling (QSL-323, being recorded in PR #534's OQ-H) is that IR names no QSL type and has no obligation identity type; only CG names it. An IR implementer who reads this doc adds the IR -> qsl-replay edge that FB-05/FB-11 forbid. Fix: "CG names this type directly; IR names no QSL type." | qsl-replay/src/identity.rs:15-16; qsl-replay/src/lib.rs:22-24 |
| FND-002 | low | The frame payload module doc still says "CG builds this payload from the frame witness bindings over IR's `WitnessBinding` (AD-016) once quire-contract-ir#109 and quire-contract-codegen#49 land". This PR makes `WitnessBinding` QSL's own. The ticket (QSL-322, FND-004 context) names this exact file. Fix: "over QSL's `WitnessBinding`", and drop or re-check the IR#109 dependency. | qsl-replay/src/witness/frame.rs:14-17 |
| FND-003 | medium | `decode` now refuses any entry that names no bound parameter (`Unbound`). But `raw_bindings` drops an entry with no `=` before `decode` sees it (`filter_map(split_once('='))`). So `<<<assertion|h|c|{x}=1;junk>>>` decodes to `Ok([1])` against `[integer(x)]`. The contract this PR adds, "every entry joins exactly one binding" (the doc comment, and ADR-013 O-25's "a binding with no parameter ... refuses"), has a silent hole. A truncated CG adapter entry is ignored instead of refused. No test covers an entry with no `=`. Fix: have `decode` refuse a `=`-less non-empty entry, as `Unbound(entry)` or a new variant, and add that row to the refusal test. | qsl-replay/src/witness.rs:124-129; qsl-replay/src/witness.rs:154-166 |
