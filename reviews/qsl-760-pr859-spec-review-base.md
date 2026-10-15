---
id: SR-11182
title: "base review of quire-spec-language PR 859 (unit-language equality on native String, UUID and Timestamp)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@83251dcbb2db420048c72a565da5b6f2cfd0478f; spec/functional/FR-056-admit-domain-package-model-declarations.md (new subsection, AC-23, AC-35, AC-36, Open Question), spec/functional/FR-091-produce-value-forms-and-assemble-package-declarations.md (type-resolution bullet, AC-40), spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md (Equality row, Native scalar equality paragraph, AC-28), spec/test-cases/TC-1028-unit-equality-on-native-string-uuid-and-timestamp.md"
review_set: subset
---

## Summary

Ticket: QSL-760. Base checklist over the edited units. Code claims were
checked against `origin/main`, and QSpec sources were read read-only on the
QSpec `origin/main`.

- Code: `BuiltinType` in `qsl-forms/src/syntax.rs` has no String, UUID or
  Timestamp head, so these names reach declaration lookup and refuse
  `UnresolvedTypeName` (`assemble.rs`). Lowering `equality()` returns
  `OperatorIneligible` for `ValueType::Uuid`, `String` and `Timestamp`
  (`lowering.rs:4063`). The new text is target design and no amended AC
  claims the current code already meets it.
- QSpec FR-322-AC-67 exists and catalogues the six `quire.op.string|uuid|timestamp.eq|ne`
  identities as binary `boolean` comparisons with no law or mode. FR-322's
  prose adds "no member and no leaves". The FR-093 row and paragraph match it,
  and match the other scalar families in the Equality row, which also carry
  no law or mode.
- QSpec FR-149-AC-14 gives same-kind UUID and Timestamp equality, and
  FR-149-AC-16 gives `operator-ineligible` for ordering. Both match the PR.
  Cross-type refusal is a different matter (FND-001).
- `include_str!` reaches FR-092, FR-093 and FR-255. The FR-093 edits add no
  `**name**: ` vector heading and no three-cell row with a 64-hex key, so
  `spec_vectors`, `spec_descriptions` and `key_table_rows` are unaffected. The
  21-vector count holds.
- The diff adds no compatibility layer. It names QSpec only by id.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Cross-type equality is given the wrong refusal cause. FR-056-AC-36, FR-093-AC-28, the FR-056 subsection and TC-1028 R1-R5/R7 refuse unlike operands (String/UUID, UUID/Timestamp, String/Text, Timestamp/Integer, UUID/Reference) as `ill_typed`/`operator-ineligible`. The owning source-equality matrix, QSpec FR-149-AC-15, refuses these pairs `ill_typed`/`type-mismatch`. The shared checker QSL calls (quire-semantic-value `check_equality_in`) also returns `TypeMismatch` for unlike types. The cited FR-322-AC-67 SE-07/SE-11 vectors are checked-package reader vectors for an identity applied to a wrong-family operand; they are not unit-language checking. Ordering (R6) is correctly `operator-ineligible` under FR-149-AC-16. | spec/functional/FR-056-admit-domain-package-model-declarations.md:386-389,835; spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md:825; spec/test-cases/TC-1028-unit-equality-on-native-string-uuid-and-timestamp.md:51-57 |
| FND-002 | medium | The FR-091 bullet says a unit declaring a form named `String`/`UUID`/`Timestamp` "makes the use an ambiguous type name, below". The Ambiguous type name rule below was not amended: it fires only when a name "names more than one alias, record, tuple or enum form of the unit", and it requires the span of each candidate. A unit with one `record UUID` has one such form, and the "exactly one record or tuple form" arm also matches. As a result, no rule yields FR-091-AC-40's `ambiguous_declaration`/`ambiguous-name` and TC-1028 step 4. | spec/functional/FR-091-produce-value-forms-and-assemble-package-declarations.md:638-643,660-663,865 |
| FND-003 | medium | Normative clauses with no failing TC step. The FR-056 subsection says a native Timestamp field SHALL have checked type `Timestamp`, and that `=` admits operands that are field reads, call results or `let` bindings. FR-056-AC-35 says operand values are supplied "as parameters or fields". TC-1028 uses parameters only, so no step reads a String/UUID/Timestamp field or checks the Timestamp field mapping. | spec/functional/FR-056-admit-domain-package-model-declarations.md:371-375,834; spec/test-cases/TC-1028-unit-equality-on-native-string-uuid-and-timestamp.md:26-29 |
| FND-004 | low | For uppercase UUID, FR-056-AC-35 names the kernel class `noncanonical-payload`, while TC-1028 E8 expects the adapter's `invalid_runtime_input`/`invalid-value`. FR-056's adapter mapping reconciles them, but the observable at a unit input is the adapter code. | spec/functional/FR-056-admit-domain-package-model-declarations.md:834; spec/test-cases/TC-1028-unit-equality-on-native-string-uuid-and-timestamp.md:44 |

## Verdict

CHANGES REQUESTED. The code claims are accurate, and the PR states target
design. The lowering identities match QSpec FR-322-AC-67, and the FR-093
edits break no include_str-parsed test. There is one high finding: the
cross-type refusal cause contradicts the owning equality matrix, QSpec
FR-149-AC-15 (`type-mismatch`). There are two mediums: the name-ambiguity
vector has no governing rule, and the field-operand and Timestamp-field
clauses have no TC step. There is one low.
