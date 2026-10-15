---
id: SR-11192
title: QSL-760 PR 862 gap-analysis
type: SpecReview
analysis: gap-analysis
scope: agent-ix/quire-spec-language@809a3d1afdb5c8cfde561ee6073eab26a809b2e5; PR 862;
  FR-056-AC-35, FR-056-AC-36, FR-091-AC-40, FR-093-AC-28, TC-1028
review_set: subset
---

## Summary

Ticket: QSL-760. A planless gap analysis of the PR's ACs against the merged FR-056, FR-091 and FR-093 text and TC-1028, the tagged tests, and the code. FR-091-AC-40, FR-056-AC-36 and FR-093-AC-28 are met, as are TC-1028 E1-E7, E9-E14, R1-R7, step 3 and step 4. The lowering identities match QSpec FR-322-AC-67.

## Verdict

FINDINGS: two high. FR-056-AC-35's unit-input refusal clause is unmet, but tests tagged FR-056-AC-35 still mark it covered. TC-1028 step 5 cannot be parsed as written.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-056-AC-35 (and TC-1028 E8) requires the uppercase UUID to be refused "at a unit input by FR-056's runtime-input adapter as `invalid_runtime_input`/`invalid-value`". FR-056 places that adapter in QSL ("The QSL runtime-input adapter SHALL map ... the three kernel payload refusals to `invalid_runtime_input`/`invalid-value`"). Nothing in this repo maps a UUID or Timestamp input: the replay `witness_type` still refuses every `Uuid`/`Timestamp` parameter as `WrongValueKind`, canonical values included, and observation `admit_leaf` has no UUID or Timestamp arm. The only E8 test asserts quire-exact's own `Uuid::from_canonical_text`, so no QSL change can make it fail. Yet four tests carry FR-056-AC-35, so the matrix shows the AC as covered. The PR body's "not in this repo" is inaccurate: the spec owns the adapter here. Either implement it, or defer it with a ticket, take FR-056-AC-35 off the trace claim until it lands, and correct the PR body. | qsl-replay/src/execute.rs:1201-1225; qsl-semantics/src/model/observation/document.rs:1191-1230; qsl-eval/tests/it/native_scalar_equality.rs:154-163 |
| FND-002 | high | Spec defect: TC-1028 step 5 names a record field `at`, and `at` is a reserved word (qsl-cst/src/token.rs:140, grammar.rs:545), so the step 5 unit does not parse. The tests rename the field `moment`. Every `at` field use in TC-1028 must be renamed: line 34 (`at: Timestamp`), F1 line 63 (`x.at = y.at`, operand label `at`, "`x.at` has checked type"), F2 line 64 (operand label `at`), F5 line 67 (`e.at`, operand label `at`), and F7 line 69 (`x.at = y.id`). FR-056, FR-091 and FR-093 contain no `at` field. `id` and `label` are not reserved, and `id` parses in the tests. | spec/test-cases/TC-1028-unit-equality-on-native-string-uuid-and-timestamp.md:34,63,64,67,69 |

## Coverage

FR-091-AC-40 is met by the six functions, the alias and result resolution, the step 4 ambiguity (code, cause, use span, record name span, `native: true`), the admission of an unused declaration, and near-miss names staying unresolved. FR-056-AC-36 is met by R1-R5 and R7 refusing `type-mismatch` at the comparison and by R6 refusing `operator-ineligible` across all 12 operator and type pairs. FR-093-AC-28 is met: each of the six identities appears exactly once, with no laws, leaves, member or mode, two arguments, and no `quire.op.text.*` node. Argument order is not asserted, which is recorded under SR-11191. The TC-1028 F1-F7 behaviour is met under the field name `moment`. All new tests except E8 can fail on pre-change code.

Plan completion: not assessed

## Examined scope

```yaml
scope:
- id: FR-056-AC-35
  path: spec/functional/FR-056-admit-domain-package-model-declarations.md
  role: examined
  excerpt: "UUID: `00112233-4455-6677-8899-aabbccddeeff` equals itself and differs from `00112233-4455-6677-8899-aabbccddeefe`; the uppercase spelling of the first is refused at a unit input by FR-056's runtime-input adapter as `invalid_runtime_input`/`invalid-value`, over the kernel's `noncanonical-payload` class, so no value of it reaches `=`."
- id: FR-056-AC-36
  path: spec/functional/FR-056-admit-domain-package-model-declarations.md
  role: examined
  excerpt: "`=` or `!=` over `String` and `UUID`, ... or `UUID` and `Reference<T>` refuses `ill_typed`/`type-mismatch` at the comparison ... `<`, `<=`, `>` and `>=` over two `String`, two `UUID` or two `Timestamp` operands each refuse `ill_typed`/`operator-ineligible`."
- id: FR-091-AC-40
  path: spec/functional/FR-091-produce-value-forms-and-assemble-package-declarations.md
  role: examined
  excerpt: "A unit declaring `record UUID { x: Integer; }` and using `UUID` as a type refuses `ambiguous_declaration`/`ambiguous-name` naming the use, the span of the record declaration and the native `UUID` candidate with no span."
- id: FR-093-AC-28
  path: spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md
  role: examined
  excerpt: "lower to one `expression` node whose operation is `quire.op.string.eq`, `.ne`, `quire.op.uuid.eq`, `.ne`, `quire.op.timestamp.eq` or `.ne`, result type `boolean`, arguments `[ref(a's node), ref(b's node)]`, and no law, no mode, no member and no leaves"
- id: TC-1028
  path: spec/test-cases/TC-1028-unit-equality-on-native-string-uuid-and-timestamp.md
  role: examined
  excerpt: "5. With `record Event { id: UUID; at: Timestamp; label: String; }`, check and evaluate the field, call-result and `let` vectors F1-F7 below."
```
