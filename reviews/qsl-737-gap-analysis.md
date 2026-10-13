---
id: SR-8694
title: "Gap analysis of quire-spec-language PR #803: FR-359 AC-1 to AC-11"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@3156a168b92fa8bb1d68c05d5832d2fa87791ef1; FR-359-AC-1..AC-11, TC-998..TC-1002, QSL-737 ACs 1-6; tests added or changed by PR #803 and the production code they exercise"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-359
    type: reviews
---
# Gap analysis of quire-spec-language PR #803

## Summary

Ticket: QSL-737. This is a planless audit of FR-359-AC-1 to AC-11 (TC-998
to TC-1002) against the computed matrix. The matrix was produced by
`quire matrix --scope <worktree> --format json` (quire 0.36.2, engine
0.50.2).

The matrix reports all 11 criteria as `tagged`:

| Criterion | Binders | `#[ignore]`d binders |
| --- | --- | --- |
| AC-1 | 4 | 0 |
| AC-2 | 1 | 0 |
| AC-3 | 5 | 1 |
| AC-4 | 2 | 0 |
| AC-5 | 3 | 0 |
| AC-6 | 3 | 0 |
| AC-7 | 3 | 0 |
| AC-8 | 5 | 1 |
| AC-9 | 5 | 1 |
| AC-10 | 2 | 0 |
| AC-11 | 2 | 0 |

Every binding read was correct. `tagged` is not the same as met, because
several criteria have halves that no passing test asserts. The findings and
the list below name each one.

Semantic review (intent to test to code, and test-oracle strength) was run by
hand on this scope, as the dispatching brief requires.

## Verdict

**FAIL**: one high finding. The snapshot half of FR-359-AC-9 is unimplemented
in this repository (FND-001). It has no external blocker, and the only test
tagged to it is a stub.

**Criteria with no passing test for part of their text:**

- **AC-1**: UUID and Timestamp field types. The integer stand-ins are blocked
  by PR #789, which is open, and no ignored test cites it.
- **AC-3**: the `event_type` node in an emitted package, and read-back
  `Verified`.
- **AC-5**: union payload (blocked: quire-exact #17 is open); no text-leaf
  comparison; nested event refusal.
- **AC-6**: "naming the field".
- **AC-8**: the model-operation path (check, emit, read-back, run, Upcast,
  dispatch), and "naming the parameter".
- **AC-9**: snapshot admission, the `Order.discount` / `Sequence` field slots
  in a snapshot, the `missing-member` cause, and the first-path-in-field-order
  refusal.

**Criteria that are met:** AC-2, AC-4, AC-7, AC-10 and AC-11. Each of their
tests goes red if the behaviour is reverted:

- Positional field reads would return `promised_by`.
- Exact `OptionValue::present` would fault.
- Exact element forming would refuse the child.
- An alias-derived key would differ between the M and N units.

**Ticket ACs:**

- QSL-737 AC-1 to AC-3 hold at evaluation.
- AC-4 holds only at the `Form::Argument` unit level.
- AC-5's kernel half depends on QSV #16, which is not merged (code review
  SR-8693 FND-001).
- AC-6 holds: `tc235_record_subtype_value_conforms_to_parent_typed_parameter`
  is un-ignored.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The snapshot half of FR-359-AC-9 is not implemented and has no external blocker. The observation reader has no FR-106 `record` value form: `read_raw_value` reads no `record` tag, and `admit_form` has no record arm. So no test covers a snapshot admitting `Order.total: Money` holding a `TaxedMoney`, the `Order.discount: Money?` payload, the `Sequence<Money>` field, `missing-member` for a `TaxedMoney` missing `tax_minor` (with a `Money` admitting without it), or the first-path-in-field-order `invalid_runtime_input`/`wrong-value-kind`. This is in-repo work inside QSL-737's scope, and the ticket's own text says it "enumerates ALL of it, no later split". The only binder is the stub in SR-8693 FND-002. | qsl-semantics/src/model/observation/document.rs:2200; spec/functional/FR-359-carry-a-child-record-value-at-an-ancestor-typed-slot.md:274 |
| FND-002 | medium | The model-operation path of FR-359-AC-8 (TC-1001 steps 1-3) is tested only by a unit test of `Form::Argument` over a hand-built `TypeEnvironment`. The shared fixture declares `Ledger.pay(m: Money): Money`, but no test calls it. Nothing checks, emits, runs or reads back an operation call with a `TaxedMoney` argument, an `Upcast` of the operation, a dispatch guard or body binding, or an operation result of declared type `Money` holding a `TaxedMoney`. Only the function-argument half is covered end to end. | qsl-semantics/src/check/state_model.rs:453-533; qsl-semantics/tests/support/child_composite.rs:293-296 |
| FND-003 | medium | The event half of FR-359-AC-3 has no test over a checked or emitted package. `records_and_events_are_model_owned_nodes_without_supertypes` asserts only `record_value_type` nodes, despite its name. The `event_type` form is checked only by the `record_form`/`record_node_key` unit test. TC-998 step 5 requires the package to be emitted with an `OrderPlaced` domain event type beside it, and `document_with_event()` exists, so a node-level assertion is possible today. | qsl-semantics/tests/it/child_composite.rs:233-286; qsl-semantics/src/check/lowering/model/tests.rs:1355-1398 |
| FND-004 | medium | No test asserts that "the comparison of a child with a different most-specific type performs no text-leaf comparison" (FR-359-AC-5, TC-999 step 3). The fixture's `Money.currency` is native `Integer`, not TC-999's `Text[3, 3; nfc]`, so no test fails if the identity check is moved after the leaf comparison. If a text field in a model record is blocked by PR #789, an ignored test should cite that blocker. Otherwise the fixture should use a text field. | qsl-semantics/tests/support/child_composite.rs:159 |
| FND-005 | low | The `reaches_event` refusal for an operand that holds an event inside a record field, an `Option` or a collection has no test, and neither has `!=` over events. Only `a = b` over two direct `Shipped` operands is asserted. | qsl-semantics/src/check/check.rs:657-686; qsl-semantics/tests/it/child_composite.rs:206-221 |
| FND-006 | low | Value-identity oracles compare `format!("{:?}")` strings instead of typed values. This covers `shown(a) == shown(b)` in the eval tests, the AC-11 "a `TaxedMoney` built in one unit equals the same value built in the other" check (two independently built values compared by Debug text, not by evaluating `==`), and the wire round-trip test. A Debug format that omits a field would let a wrong value pass. | qsl-eval/tests/it/child_composite.rs:189-191; qsl-eval/tests/it/child_composite.rs:782-786; qsl-replay/src/result/wire/value.rs:202-212 |

## Coverage

- Plan completion: not assessed.
- Matrix: FR-359-AC-1 to AC-11 are all `tagged`; 3 binders are `#[ignore]`d
  (AC-3, AC-8, AC-9).
- Ignored tests and their blockers:
  - `a_child_record_argument_reads_back_verified`: the blocker has cleared.
    quire-contract-ir #376 is merged and IR-770 is Done; see SR-8693 FND-003.
  - `a_child_record_in_a_snapshot_admits_at_a_parent_typed_field`: no
    external blocker. It is a stub (FND-001; SR-8693 FND-002).
- Declared blockers that are real:
  - quire-semantic-value #16 is OPEN.
  - quire-spec-language #789 is OPEN (Timestamp/UUID leaves).
  - quire-exact #17 is OPEN (union carrier).
- Reverse gap: no production change lacks an owning requirement. Each new
  mechanism traces to FR-359 items 1-9 or to FR-094.
- Stubs: one test stub (above). There are no production stubs and no
  `todo!`.

## Dispositions

Round 1, reviewed at `eb90798d0e`.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 5f1fdc74a, plus 0045fc336 for parameters and results. The reader adds a `record` tag on the explicit heap stack (exactly `type` and `fields`), and `RawNode`/`RawForm`/`SnapshotValue` gain a record variant. `admit_record` resolves `type`, checks `composite_conforms` against the slot and requires exactly the effective fields (`missing-member`, `unknown-member`). Snapshot tests cover the TaxedMoney and GiftTaxedMoney admissions, the `discount?` payload, the `Sequence<Money>` field, ancestor, unrelated, object and unknown `type`, a reference in place of a record, a missing `tax_minor`, an extra field, a malformed scalar, and first-path-in-field-order. Each test fails if `admit_record` is reverted, because a `record` value would then refuse `wrong-value-kind` at the field. The path encoding and the depth bound are new findings in SR-8693. |
| FND-002 | fixed | 0045fc336: `a_model_operation_takes_and_returns_a_child_record` admits a `Ledger.pay` invocation whose argument and result are both `TaxedMoney`, checks that both keep their declaration, and evaluates the pre and post clauses. A `Money` or `Fee` at `payTaxed(m: TaxedMoney)` refuses `wrong-value-kind` with `field=m`. `an_operation_over_a_record_parameter_emits_and_reads_back` reads back `Verified`. `dispatch_argument_admits_a_conforming_record_and_refuses_the_rest` covers dispatch arguments at check level. The run-level dispatch and Upcast remainder is FND-007. |
| FND-003 | fixed | c764dff09: `an_event_type_is_one_model_owned_event_type_node_in_a_package` asserts exactly one `event_type` node owned by `(model, PACKAGE, Shipped)`, beside an `OrderPlaced` record node, with no `composite_type` and no `supertypes`. |
| FND-004 | deferred | Blocked by PR #789 (QSL-723), which is open: String leaves do not lower yet. `a_mixed_pair_with_equal_text_leaves_is_unequal` is `#[ignore]`d with that citation and asserts a `StampedTicket` and a `Ticket` with an equal `label` are unequal. Un-ignore it when #789 merges. |
| FND-005 | fixed | 3a3f6e00c: `equality_over_event_operands_stays_refused` covers `=` and `!=` over two events, a record holding an event, `Option<Shipped>` and `Sequence<Shipped>`. Each case asserts `operator-ineligible` and the comparing function's body. |
| FND-006 | fixed | c764dff09: the oracles use `quire_exact::compare_keys(..) == Some(Equal)` (`same_value`), which compares declaration and fields. The AC-11 test adds negative controls (`!same_value(taxed, money)` within and across units), so the oracle can fail. The wire round-trip compares `compare_keys`. |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-007 | low | Part of FR-359-AC-8 still has no test: a run through an `Upcast` of the operation, and a dispatch guard or body binding that binds a child argument, are not exercised at run level. Dispatch argument admission is tested at check level only, and the Upcast not at all. The production path (the `Form::Argument` conformance arm, then evaluation by `record_slot`) is the one already exercised for functions and operation clauses, so the risk is low. AC-8 stays partly unbacked until a run-level test is written with the TC-196 dispatch harness. The PR body declares this gap. | qsl-eval/tests/it/dispatch_calls.rs; qsl-semantics/src/check/state_model.rs |
