---
id: SR-8693
title: "Code review of quire-spec-language PR #803: child composite values at parent-typed slots"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@3156a168b92fa8bb1d68c05d5832d2fa87791ef1; base 2e87d77a4 (impl/business-model-meanings); qsl-semantics/src/check/{check,assemble,ir,facts,field_refinement,lowering,lowering/model,state_model,mod}.rs, qsl-semantics/src/model/observation/document.rs, qsl-eval/src/value/expression/evaluate.rs + evaluate/invariants.rs, qsl-replay/src/execute/argument.rs, qsl-replay/src/result/wire/value.rs, Cargo.lock, spec/functional/FR-094, all added/changed tests"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-359
    type: reviews
---
# Code review of quire-spec-language PR #803

## Summary

Ticket: QSL-737. Code review with the Rust lane (rust-review) folded in, over
the six commits of PR #803 on top of `impl/business-model-meanings`. The
design follows FR-359: a checked field read carries `RecordField{declaration,
name}` and evaluation resolves the slot in the value's own declaration (with a
rename table for FR-082 redefinitions under another name); `Typer::coerce` and
`Form::Argument` admit a record by `composite_conforms`; equality keeps one
static type and refuses an operand reaching an event type as
`operator-ineligible`; `project`, replay decode and replay argument conversion
admit Option payloads and collection elements with `TypeEnvironment::admits`
and build with `from_admitted`/`form`/`from_admitted_slots`; model record
value types and event types are emitted as `model`/`record_value_type` and
`model`/`event_type` nodes and their type-environment declaration key is the
model node key (no alias, no source owner).

Gates were not run (reviewer-only brief: no builds). The committed head was
checked statically against its own Cargo.lock.

## Verdict

**FAIL**: one high finding. The committed head does not compile against its
own lock (FND-001), because it calls the QSV #16 API and QSV #16 is open and
not in the lock. The PR body declares this. It still blocks the merge.

Checked and clean:

- The three defects of the reverted QSL-732 `Typer::coerce` attempt do not
  return. Field reads are by identity, not position: `NodeKind::Field` has no
  index, and `Scope::record_slot` finds the slot by name in the value's own
  declaration. Equality reaches `CheckedEquality::evaluate` with the type
  environment. Optional projection admits by `types.admits` instead of the
  exact `OptionValue::present`, and `field-option-payload-rejected` stays for
  a non-conforming payload.
- `Child == Parent` over mixed static types stays `ill_typed`. The equality
  check does not go through `coerce`.
- There is no conversion node and no `supertypes` member.
- `record_node_key` keys a record by `ModelOwner` and form only, with no alias
  and no supertypes.
- The record and tuple build path uses `TypeEnvironment::record`/`tuple`
  (environment admission) plus `retain_composite`, never `evaluate_record`.
- Replay decodes by the record's own declaration, checks
  `composite_conforms`, and compares field names in order.
- The FR-094 text update for the two new node forms is correct and
  consistent with FR-359 items 3 and 9.
- No new `unwrap`/`panic` in production code, and no `unsafe`.
- No new string-literal comparison falls under the string-edge lint
  (`split_once` is not a scanned method, and the new comparisons are
  binding-to-binding).
- No vendoring, no file or version tracking, nothing that must not land in a
  public repo.
- The event-equality refusal is FR-359-backed: item 4 says "`==` over event
  types stays refused" and AC-5 says "An `==` over two event-typed operands
  stays refused", grounded in QSpec FR-208, which gives event types no
  equality.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The committed head does not compile. evaluate.rs calls `checked.evaluate(self.scope.types(), &left, &right, meter)` and `operand_value(self.scope.types(), operand, ...)`, and the eval tests call `evaluate(&env, ...)`. Cargo.lock pins quire-semantic-value `4ff3c61`, where `CheckedEquality::evaluate(&self, left, right, meter)` and `operand_value(operand, value, units, meter)` take no environment. That API is QSV PR #16, which is still OPEN. Merging #803 as it stands turns its base red. QSV #16 must merge first and the lock bump must land in this PR. | qsl-eval/src/value/expression/evaluate.rs:1364; qsl-eval/src/value/expression/evaluate.rs:1640; Cargo.lock:1665 |
| FND-002 | medium | The ignored test `a_child_record_in_a_snapshot_admits_at_a_parent_typed_field` is a stub tagged `#[trace("TC-1002", "FR-359-AC-9")]`. It parses a JSON literal, asserts the parse succeeded and then unconditionally `panic!`s, and it calls no production code. Its doc comment says it "asserts today's refusal so it fails once the reader lands". That is false: it fails whatever the reader does. As written it inflates AC-9 coverage and cannot become a passing test without being rewritten. | qsl-semantics/src/model/observation/document.rs:2188-2205 |
| FND-003 | medium | `a_child_record_argument_reads_back_verified` stays `#[ignore]` with the reason "until quire-contract-ir lands IR-770 and FR-347". Both of those are done: quire-contract-ir PR #376 is MERGED (d52d5f1) and IR-770 is Done. The lock pins quire-contract-ir `eb007b2`, which predates #376. The cited blocker has cleared, so this PR should bump the lock and un-ignore the test. Until then the AC-3 and AC-8 read-back halves have no passing test. | qsl-package/src/emit/tests/child_composite.rs:115; Cargo.lock:1597 |
| FND-004 | medium | The refusal oracles assert the cause and nothing else. `refused_as_mismatch` passes when every refusal is `TypeMismatch`, wherever it is reported. FR-359-AC-6 requires "naming the field", AC-8 requires "naming the parameter" and TC-999 step 5 requires "at the operands", and no assertion checks any refusal's location or path. A `type-mismatch` raised at the wrong site, such as the callee body or another argument, still passes. | qsl-semantics/tests/it/child_composite.rs:75-82 |
| FND-005 | low | Every field read during evaluation allocates and scans. `ModelRecords::own_name` builds `field.name().to_owned()` to look up a `(NodeKey, NodeKey, String)` BTreeMap key, even when the rename table is empty, and then `record_slot` scans the field names linearly. Before this PR, `Field` used an O(1) index. Every `e.f` in a hot loop now pays a heap allocation. A fix: short-circuit when `renamed` is empty, and use a borrowed-key lookup. | qsl-semantics/src/check/check.rs:373-381; qsl-semantics/src/check/check.rs:693-704 |
| FND-006 | low | `ModelRecords::rename` and `ModelRecords::declare_event` are `pub` but have no caller outside qsl-semantics, where only `assemble.rs` calls them. `pub use check::ModelRecords;` was inserted inside the comment block that documents the test-support re-exports, so the comment now misdescribes it. | qsl-semantics/src/check/check.rs:350; qsl-semantics/src/check/check.rs:362; qsl-semantics/src/check/mod.rs:158-161 |
| FND-007 | low | `close_composite` finds a model record's occurrence root by splitting the display name `alias::Artifact` with `split_once("::")`, which recovers identity from a formatted string. A name without `::` silently records no occurrence root, with no refusal. The alias is known at assembly, so it could be carried with the record's declaration. | qsl-semantics/src/check/lowering.rs:1995-2005 |

## Dispositions

Round 1, reviewed at `eb90798d0e` (fix commits 3a3f6e00c..eb90798d0 on top of 3156a168b).

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 1f6fe99c1: Cargo.lock now pins quire-semantic-value 8e21d505, which is QSV #16's merge commit and QSV main. That revision declares `CheckedEquality::evaluate(&self, types, left, right, meter)` and `operand_value(types, operand, value, units, meter)`, matching the calls. quire-contract-model is pinned at d52d5f1b (contract-ir #376). Its one public API break against eb007b2 is the required `CompleteLoweringProfileV2::reference`, adapted in extent_agreement.rs, and no lib.rs export was removed. No `.cargo` patch is committed. Compile-cleanliness was checked by reading the code, not by building. |
| FND-002 | fixed | 5f1fdc74a: the stub is deleted. Real reader tests (`a_record_value_reads_by_its_type_and_fields_in_document_order`, `collect_references_walks_into_a_record`) and four snapshot admission tests replace it. |
| FND-003 | fixed | 1f6fe99c1: the contract-ir lock is bumped to d52d5f1b and `a_child_record_argument_reads_back_verified` is un-ignored. |
| FND-004 | fixed | 3a3f6e00c: `refused_at` asserts one refusal, its cause, the function body that raised it, and the child-index path. A new case checks that the refusal names the second argument and the `discount` field. |
| FND-005 | fixed | 3a3f6e00c: the rename table is now keyed `(value, read_at) -> name -> own`, so a read does a borrowed lookup and allocates nothing. The remaining linear scan over a record's fields is accepted; it was not the defect. |
| FND-006 | fixed | 3a3f6e00c: `rename` and `declare_event` are `pub(crate)`, and `ModelRecords` is re-exported in the `pub use check::{..}` list instead of the test-support comment block. |
| FND-007 | fixed | 3a3f6e00c: the occurrence root's alias comes from the `AdmittedModel` that holds the record (`model.alias()`). A missing alias is now a typed `UnknownDeclaration` fault instead of a silently skipped root. |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-008 | high | Record admission recurses natively and refuses anything deeper than an invented `MAX_RECORD_DEPTH = 256` with `invalid-value`. FR-106 says "Value validation SHALL use an explicit work stack ... finite recursive record declarations therefore admit only finite supplied values within the byte and value limits, with no call-stack or schema-depth limit", and FR-261 says reads report the document "whatever its nesting". Example: a recursive record `Node { next: Node? }` supplied 257 levels deep, within `observation.values` and `input_bytes`, refuses `invalid-value`, which FR-106 reserves for a malformed scalar. The bound has no specification, no test and no catalogued cause. The PR body says FR-261 does not cover admission, but FR-106's own Behavior text does. | qsl-semantics/src/model/observation/document.rs:880-884; qsl-semantics/src/model/observation/document.rs:1219-1222 |
| FND-009 | medium | The nested refusal path is a rendered dotted string. `AdmissionRecord::within` prepends `step.` to a `path` text field. FR-106 requires "its exact FR-207 field, option_value and index steps", and QSpec FR-207 says a locus is "an ordered sequence of typed traversal steps rather than ... a rendered path string". Three defects follow. A `present` wrapper adds no `option_value` step: `discount: {present: TaxedMoney without tax_minor}` reports `field=discount, path=tax_minor`, the same as a required field. An index is added only when the element is a record, so a scalar element defect inside a record's sequence field has no index. A field named `0` cannot be told apart from index 0. | qsl-semantics/src/model/observation.rs:362-374; qsl-semantics/src/model/observation/document.rs:1098-1108 |
| FND-010 | low | `References::record_type` keys the snapshot record with `IdentityLimits::default()`, but assembly declares the type-environment key under the unit's configured `limits.identity`. With non-default identity limits, a record that assembled fine can fail to key at admission and refuse `wrong-value-kind`. It also scans every record of every selected view, linearly, for each record value. | qsl-semantics/src/model/observation/document.rs:931-942 |
