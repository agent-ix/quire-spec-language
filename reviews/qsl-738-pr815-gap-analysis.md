---
id: SR-9021
title: "Gap analysis of quire-spec-language PR #815: FR-360-AC-1 to AC-11 and FR-056-AC-27 to AC-33 against their tests"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@bddce547903993dcb29a7fe391d6a390a209a45b; FR-360-AC-1..AC-11 (TC-1006..TC-1010), FR-056-AC-27..AC-33 (TC-997, TC-1005) against qsl-semantics/tests/it/variant_fields.rs, qsl-semantics/tests/it/variant_type.rs, qsl-eval/tests/it/variant_fields.rs, qsl-replay/src/execute/tests/variant_fields.rs, qsl-semantics/src/model/observation/document.rs tests, qsl-replay/src/spine/request.rs tests"
review_set: subset
---
# Gap analysis of quire-spec-language PR #815

## Summary

Ticket: QSL-738. Planless. Each acceptance criterion was mapped to the tests tagged with it. Each test was judged by mutation reasoning: would it fail if the production change it claims were reverted? No red log exists (the coder wrote code and tests together), so mutation reasoning is the measure. Plan completion: not assessed.

Examined:
- FR-360-AC-1: TC-1006 steps 1 and 3, `a_member_typed_by_a_variant_type_is_admitted_with_a_field_export_only`, `an_event_admits_a_variant_field_but_not_a_variant_occurrence`. This would fail if the intake refusal were restored. The `Settings` record export listing `tier` is not asserted. (examined)
- FR-360-AC-2: `the_assembler_types_variant_members_by_the_enum_type` and `the_keys_of_a_variant_type_do_not_depend_on_the_alias`. These would fail without the `model_value_type` Enum arm or with an alias-dependent key. (examined)
- FR-360-AC-3: `a_variant_type_with_no_case_refuses_once_and_its_field_raises_nothing` covers sentence 1. Sentence 2 has no test and no code; see SR-9022 FND-001. (examined)
- FR-360-AC-4: `a_case_literal_and_every_variant_producing_form_check_as_the_variant_type`, which uses `check_expression` over a graph naming no variant type. This would fail without `Typer::variant_case`. (examined)
- FR-360-AC-5: `each_mistyped_use_of_a_variant_type_refuses_with_its_cause` and `an_unknown_variant_type_name_in_a_type_position_refuses_at_the_assembler` (examined)
- FR-360-AC-6: the three qsl-eval equality tests (examined)
- FR-360-AC-7: `records_compare_their_variant_fields_by_case`, the two Set-order tests, and `option_and_sequence_payloads_compare_by_case` (examined)
- FR-360-AC-8: `a_call_with_a_value_of_another_variant_type_refuses_at_the_argument`, `the_emitter_refuses_a_variant_typed_parameter_at_its_type_node`, and the TC-997 signature admission tests (examined)
- FR-360-AC-9 and AC-10: the snapshot and invocation admission tests, `a_variant_value_reads_its_members_as_written`, and `parameter_value_reader_keeps_the_members_of_a_variant` (examined)
- FR-360-AC-11: the two qsl-replay witness tests (examined)
- FR-056-AC-27, AC-28 and AC-32: the reworked TC-997 tests in variant_type.rs. FR-056-AC-33 (TC-1005): the deleted tc235 test is consistent with the merged AC text. (examined)

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Three FR-360-AC-5 refusals that the emitter does not block have no test. The Text literal `"Gold"` at a `Config::Tier` slot (only `0` is tested), `none` at a `Config::Tier` slot, and a `case` expression over a `Config::Tier`, refused at the scrutinee. `check_expression` reaches all three, as it does the cases already tested. Fix: add the three rows to `each_mistyped_use_of_a_variant_type_refuses_with_its_cause`, asserting the cause and, for `case`, the scrutinee path. | qsl-semantics/tests/it/variant_fields.rs:487-597 |
| FND-002 | medium | Several FR-360-AC-7 nested-equality cases are untested: two `Option<Config::Tier>` (both `none`, or the same and different cases), `Set<Config::Tier>` equality, and a tuple position. These are evaluation-only and not blocked by the emitter. `option_and_sequence_payloads_compare_by_case` says in its name and doc comment that it covers `Option<Tier>`, but it evaluates only two `Sequence` pairs. Fix: add the `Option`, `Set` and tuple pairs (true and false each), or rename the test to what it covers. | qsl-eval/tests/it/variant_fields.rs:429-471 |
| FND-003 | medium | Parts of FR-360-AC-4, AC-8, AC-9 and AC-11 are untested or unimplemented in this PR. All of them need a unit that names a variant type to check, run or replay, which the interim emitter refusal blocks (item 12). AC-4: `f(t: Config::Tier)` and a call to it, `Config::Tier::Gold` at an `Account.tier` slot and in an `Option` literal, a clause naming the type, and a `Gold Plus` variant type. A `Gold Plus` type is refused (`UnsupportedVariantType`/`UnsupportedModelMember`) where AC-4 and QSpec FR-141-AC-8 admit it. AC-8: "checks and runs", and the emission shape. AC-9: replay through a run. AC-11: no encoder writes the item 9 form (QSL has only readers of it), and the witness encode side and the digest round trip are untested; only witness decode is tested. Linear has the edge QSL-738 blocked-by QSL-752 (contract-ir v2, and the QSV enumeration for non-identifier case names). Disposition sought: deferred to QSL-752, with QSL-738 kept open. The item-10 encoder is QSL-738's own remaining work, not QSL-752's. | qsl-semantics/tests/it/variant_fields.rs:419-480, 1043-1053; qsl-replay/src/execute/tests/variant_fields.rs:127-173 |
| FND-004 | low | The item 9 rule that `missing-member` reports `type` first is not pinned. No test sends `{"variant":{}}`, and no test asserts the `member` detail (`type` or `case`) that `admit_variant` attaches. Swapping the two would pass every test. Fix: add `{"variant":{}}` expecting `missing-member` with member `type`, and assert the member on the two single-missing rows. | qsl-semantics/src/model/observation/document.rs:1212-1225; qsl-semantics/tests/it/variant_fields.rs:876-886 |
| FND-005 | low | `the_emitter_refuses_a_variant_typed_parameter_at_its_type_node` asserts only one refusal with cause `type-mismatch`. It does not assert the location its name and FR-360-AC-8 give, the parameter's type node. A refusal from anywhere else in `pick` would pass. Fix: assert the refusal's location or origin as `refused_in_body` does. | qsl-semantics/tests/it/variant_fields.rs:1043-1053 |

## Verdict

**CONDITIONAL**: no high findings. FND-001, FND-002, FND-004 and FND-005 are cheap and can be fixed in this PR. FND-003 is deferred work with an owner, and QSL-738 must not close on this PR.

`quire matrix` at this head lists FR-360-AC-1..AC-11 and FR-056-AC-27..AC-33 all as `tagged`. Being tagged does not mean the criterion is covered: AC-3 sentence 2, and the parts of AC-4, AC-8, AC-9 and AC-11 in FND-003, are tagged but not exercised.

Test-oracle strength, by mutation reasoning:
- Intake, assembler, literal resolution, cross-type refusal, ordering refusal, equality, Set order and replay admission order: each test fails if its production arm is removed or reordered. The Set-order test over `Grade` (declared `Standard, Gold, Bronze`) fails if the rank follows declaration order. The admission-order rows "missing case beats wrong type", "extra beats undeclared" and "wrong type beats undeclared" pin item 9's (b) > (d), (c) > (e) and (d) > (e).
- The alias-independence tests compare effective identity, node key, shape and per-case keys under `Config` and `Cfg`, and replay compares admitted values under both aliases.
- All tests are typed: causes, codes, paths, keys and values, never message text.

Known gaps the brief named:
- (1) Emitter: deferred to QSL-752 (FND-003). The tests are honest about it. The semantics `Replay` and qsl-eval `package_as` doc comments say the unit names no variant type, the emitter test names the interim, and the invocation tests use hand-built `ClauseFacts`. Those facts carry a placeholder identity, `NodeKey::from_digest([7; 32])`, which is a test fixture and is not claimed as a checked clause.
- (2) `Gold Plus`: deferred to QSL-752 part 2 (FND-003). The refusal codes are catalogued (SR-9020).
- (6) AC-7 `Option` and tuple, and AC-11 round trip: FND-002 and FND-003.
- (7) spec/tests.md markers: no change is warranted. None of TC-1006..TC-1010 is fully covered, and TC-994..TC-997 from the merged QSL-734 are still 🚧 too. The computed matrix is the coverage authority, so flipping a hand-kept marker adds nothing.

Bindings checked: the TC-997 tests keep FR-056-AC-27, AC-28 and AC-32. `a_later_parameter_naming_a_forbidden_meaning_keeps_its_own_position` keeps the remaining refusal path and position at params[1] with a state machine, so FR-056-AC-32's "position 1 for the second parameter" is still pinned after the variant case moved to admission.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-006 | medium | The evidence test `the_text_none_and_case_spellings_have_no_form_to_check` passes on any parse or build failure and asserts no cause. Its `case` spelling, `case Config::Tier::Gold { Gold: true }`, is not the grammar's form (`case <ident> when <block> <control>`, grammar.rs:1196-1204), so that row fails for syntax and proves nothing about a missing form. The source also selects no model. The claim itself is true by inspection: the grammar parses `none` and text literals, but `ExprNode` has no form for them or for `case`. Fix: use the grammar's spellings with the model selected, and assert the specific S1 or S2 refusal (code and cause) that each spelling gets. | qsl-semantics/tests/it/variant_fields.rs:1288-1316 |
| FND-007 | low | The digest check compares two hand-written JSON values, which JCS canonicalization always makes equal. It never hashes the encoder's output, so the encoder could reorder or drop members and still pass. The typed decode-encode-decode check on `history` is real. Fix: hash `encoded.to_json()` against the admitted input document's value for the same field. | qsl-semantics/tests/it/variant_fields.rs:1401-1409 |

## Dispositions

Round 1, reviewed at 101dbdf0809546eae22d2047c429a58f805ff612.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | rejected | The finding assumed `check_expression` reaches a Text literal, `none` and `case`. It does not: the qsl-forms `ExprNode` has no Text, None or Case variant (syntax.rs:488-700), so no checker row can exist at this ref. The real defects are new FND-006 (weak evidence test) and SR-9022 FND-007 (spec still requires check-stage refusals). |
| FND-002 | deferred | The `Option` and `Set` equality tests were added in 101dbdf08: `an_option_of_a_case_compares_by_case` (both none, none/Gold, Gold/none, Gold/Gold, Gold/Standard) and `a_set_of_cases_compares_by_case_whatever_its_build_order`. The misnamed test was renamed to `a_sequence_of_cases_compares_by_case_in_supply_order`. Tuple-position equality needs a unit that declares a tuple naming the variant type, and the interim emitter refuses that declaration. `a_tuple_naming_a_variant_type_is_held_back_by_the_emitter` pins the interim. Deferred to QSL-752. |
| FND-003 | deferred | To QSL-752 (Linear edge: QSL-738 blocked-by QSL-752); QSL-738 stays open. Remaining: AC-4 (`f(t: Config::Tier)` and a call to it, a literal at an `Account.tier` slot and in an `Option` literal, a clause naming the type, and a variant type with a `Gold Plus` case); AC-7 tuple position; AC-8 check-and-run, emission and `Verified` read-back; AC-9 replay through a run; AC-11 counterexample witness encode and the digest round trip of an encoded run result. The item 10 encoder has landed (FR-360-AC-11 partly covered). |
| FND-004 | fixed | 101dbdf08 |
| FND-005 | fixed | 101dbdf08 |
