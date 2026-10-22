---
id: SR-12310
title: 'Code review of PR #1198: node-row argument count and type checks against the attempt binder record (QSL-887, FR-147-AC-14)'
type: SpecReview
analysis: code-review
scope: 'agent-ix/quire-spec-language@41fbf4af8; git diff origin/main...41fbf4af8 (PR #1198, branch qsl-887d-impl): qsl-semantics/src/check/refinement.rs, qsl-semantics/src/check/refinement/internal_list.rs, qsl-semantics/src/check/mod.rs, qsl-replay/src/spine/refinement.rs, qsl-semantics/tests/it/refinement_protocol.rs; ticket QSL-887'
review_set: subset
date: '2026-10-22'
relationships:
- target: ix://agent-ix/quire-spec-language/FR-147
  type: reviews
---

## Summary

Ticket: QSL-887. PR: quire-spec-language#1198, head 41fbf4af8. Method: code-review with the rust-review lane folded in. Tooling: quoin 0.28.3, spec-artifacts-process@b82e42e.

The PR builds `ProtocolClasses::records`, a map from each `attempt` node name to its binder record (the receiver's reference type, then the operation's parameter types, then its result type when it has one). `node_target` refuses a written list whose length differs from the record `ill_typed`/`operator-ineligible` at the node name, and refuses an argument that does not conform to its lifted field type `ill_typed`/`type-mismatch` at the argument's span. An absent list is padded to the record length with open fields. `RefinementCause` gains `NodeArgumentCount` and `NodeArgumentType`. `ProtocolView` carries the protocol's `BoundAttempt`s, index-aligned with `checked_protocols` (both pushed at the same point in `check::mod`).

Measured at head:

- `cargo test -p qsl-semantics --all-features --test it refinement_protocol`: 20 passed.
- Mutations: disabling the count check, the type check, or the absent-list padding each fails its AC-14 test (all three fail together, 3 of 3). Removing the parameter and result fields from the record derivation survives: all 69 `refinement` tests pass (FND-001).
- `make string-edge`: pass. `make arch-lint-api-surface-qsl`: every rule PASS. `cargo clippy -p qsl-semantics -p qsl-replay --all-features --all-targets -- -D warnings`: exit 0.
- The PR body carries the loud-break changelog line for the two new public `RefinementCause` variants (the enum is not `#[non_exhaustive]`).

Judgement calls checked against the spec text, not the coder's account:

- "at the node" is the node name's span: the count refusal is at `node.span` (the `NameForm` of `Spec::Twice::a1`), which the test asserts.
- An empty list: the grammar (qsl-cst/src/grammar.rs:2116-2123) admits no `()`, and QSpec FR-432 says "The grammar admits no empty list `()`" and "A row with no argument list leaves every field open". So `arguments.is_empty()` is always the absent list, and treating it as all-open is right.
- `operator-ineligible` for a count mismatch is the cause FR-147 states ("An argument list whose length differs from the field count SHALL refuse `ill_typed`/`operator-ineligible` at the node") and the cause QSpec FR-432 states ("as an operation application with the wrong number of arguments does").
- The record order and types match ADR-027 ST-1 ("the binder's record (receiver, arguments, result)") and QSpec FR-432's attempt row (receiver of the type the operation name names, each parameter's declared type, the declared result type). `BoundAttempt::receiver` is the type `on M::T::op` names, not the declaring supertype, as FR-432 requires.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| `node_target` count and type checks | qsl-semantics/src/check/refinement.rs:2211-2238 | examined |
| `RefinementCause::NodeArgumentCount`, `NodeArgumentType` and their code/cause | qsl-semantics/src/check/refinement.rs:253-337 | examined |
| `ProtocolClasses::of` record derivation and `record` | qsl-semantics/src/check/refinement/internal_list.rs:440-545 | examined |
| `ProtocolView` construction | qsl-semantics/src/check/mod.rs:2150-2167 | examined |
| `check_limit` match arms | qsl-replay/src/spine/refinement.rs:294-299 | examined |
| AC-14 tests | qsl-semantics/tests/it/refinement_protocol.rs:1072-1180 | examined |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The parameter and result fields of the attempt binder record are untested. Every AC-14 fixture uses `inc`, which has no parameter and no result, so the record is the receiver alone. Deleting `.chain(operation.parameters()...)` and `.chain(operation.result().cloned())` from `ProtocolClasses::of` leaves all 69 `refinement` tests green. Add a fixture with an attempt on an operation with a parameter and a result (QSpec FR-432-AC-14's `a2` on `quad(x: Int[0, 9])` with result `Int[0, 99]`): `(self, 1, _)` admitted, `(self, 1)` refused with expected 3 / actual 2, `(self, true, _)` refused `type-mismatch` at `true`. | qsl-semantics/src/check/refinement/internal_list.rs:447-456 |
| FND-002 | low | The type-mismatch test asserts `expected` is `ValueType::Reference(_)` of any type. AC-14 says it is the reference type of `Spec::Counter`. Assert the id. | qsl-semantics/tests/it/refinement_protocol.rs:1175-1178 |

## Verdict

No major code defect. The count check, the type check, their codes, causes, loci and retained fields match FR-147 and QSpec FR-432 for the `attempt` kind, and each is killed by mutation. FND-001 (medium) and FND-002 (low) are coverage gaps. The blocking gap of this PR is in the gap analysis (SR-12311 FND-001): the six other node kinds get no check although QSpec FR-432 states their records.

## Dispositions

| FND | outcome | sha |
| --- | --- | --- |
| FND-001 | fixed: Added `quad(x)` with a result and `Evt{p,q}` to the Spec fixture and tests asserting 3-field `a2` rows, count 3/2 and type mismatch at `true` | 3a5180722 |
| FND-002 | fixed: The mismatch test asserts `expected` is `Reference` of the id `Twice`'s role `w` acts on (`Spec::Counter`) | 3a5180722 |

### Reviewer disposition pass 1

Re-checked by the reviewer at head 46b653ebd (fix commit 3a5180722, dispositions b49740b2d, then a merge of origin/main). These rows are the measured outcomes and supersede the coder's rows above where they differ.

Measured: `cargo test -p qsl-semantics --all-features --test it refinement` 72 passed. Deleting the parameter and result `.chain` calls from `ProtocolClasses::of` now fails three tests (`the_binder_record_has_a_field_per_parameter_result_and_record_field`, `the_argument_count_is_the_field_count_of_every_node_kind`, `an_argument_of_the_wrong_type_is_refused_at_its_field`).

The new `quad` fixture uses plain `Integer` where QSpec FR-432-AC-14 uses `Int[0, 9]` and `Int[0, 99]`. The bounds do not change any outcome the QSL tests assert: the field count is the same, and `true` against an integer field is a type mismatch either way. Only FR-432-AC-15's retained `expected` value (`Int[0, 9]`) depends on the bound, and that AC is QSpec's (TC-386); FR-147-AC-14 names only `a1` over `inc`. The bounded types are not needed here.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 3a5180722 |
| FND-002 | fixed | 3a5180722 |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-003 | medium | The `effect`, `commit` and `receive` records are untested. The `Rec` fixture declares no `effect` or `commit` node, and its only `receive` row is `rx(_)`, which is admitted with or without a record. Removing `Effect`, `Commit` or `Receive` from the match in `ProtocolClasses::of` leaves every `refinement_protocol` test green (measured; `event`, `finish`, `send` and the record-field expansion are each killed). Add an `effect` and a `commit` node to `rec()` with a count refusal each, and `rx(true, true)` refused 1/2. | qsl-semantics/src/check/refinement/internal_list.rs:479-488 |
| FND-004 | low | The type-mismatch rows over `Rec` assert the position and locus but not `expected`/`actual` (`ev(true, true)` should retain the integer field type and `Boolean`), so a wrong field type with the same mismatch would pass. | qsl-semantics/tests/it/refinement_protocol.rs:1281 |
