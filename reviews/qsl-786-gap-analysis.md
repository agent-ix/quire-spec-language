---
id: SR-12043
title: "QSL-786 PR #921 gap analysis: native scalar literals against FR-091, FR-056, FR-093 and TC-1052"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@ba47de62637888e49170fb6561d9559a26e65c8a; qsl-forms/tests/it/value_forms.rs, qsl-semantics/tests/it/native_literals.rs, qsl-eval/tests/it/native_literals.rs, qsl-replay/src/execute/tests/native_scalar.rs, spec/functional/FR-091-produce-value-forms-and-assemble-package-declarations.md, spec/functional/FR-056-admit-domain-package-model-declarations.md, spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md, spec/test-cases/TC-1052-native-string-uuid-and-timestamp-literals.md"
review_set: subset
---

## Summary

Ticket: QSL-786. PR quire-spec-language#921 at head ba47de626. Planless gap analysis of FR-091-AC-41, FR-091-AC-42, FR-091-AC-43, FR-056-AC-40, FR-056-AC-41, FR-093-AC-29 and TC-1052 against the tagged tests and the code. Plan completion: not assessed. Read-only: tests were read, not run. Major gaps only, as briefed.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The S2 vectors of FR-091-AC-41 replace the escape spellings with raw characters. `"é"`, `"é"` and the surrogate pair `"😀"` are written as raw `é`, `e`+U+0301 and the emoji, so the `\uXXXX` escape path is exercised in S2 only by `\u0000`, and the surrogate-pair decode (one scalar U+1F600) is exercised nowhere. TC-1052 step 1 names these exact bodies. | qsl-forms/tests/it/value_forms.rs:1094-1104 |
| FND-002 | medium | Several stated vectors and outputs are untested: FR-056-AC-41 `r = "abc"` over `r: Reference<T>`; FR-091-AC-42 `M::UUID("...")` over an alias with no `UUID` refusing `missing-name`, and the native candidate of the ambiguity (see SR-12042 FND-001); FR-093-AC-29 `quire.op.uuid.eq` (the test uses `!=` and asserts `quire.op.uuid.ne`), the `semantic_type`/body `type` node of each literal, no mode or member on the equality, and no text-profile definition in the lock evidence. The FR-056-AC-41 rows also assert the cause but not the stated location (comparison vs literal). | qsl-semantics/tests/it/native_literals.rs:126-160, qsl-semantics/tests/it/native_literals.rs:278-302, qsl-semantics/tests/it/native_literals.rs:304-336 |

## Verdict

PASS (no high findings). Both findings are oracle-strength gaps on a correct implementation; the main-path oracles are exact.

Covered with exact oracles: S2 `ExprNode::String` scalar sequences for the raw and `\n`/`\u0000`/`\"\\\/` spellings, and no normalization; `UUID`/`Timestamp` building ordinary `Call` forms; the lexer refusals `invalid-escape` and `invalid-token` (including `Timestamp(007)`); all seven `MalformedUuidLiteral` texts with the argument, code and call location; both `IntegerOutsideI128` overflows with site, value, `Max` limit and paths `[0]`/`[0, 0]`, and the in-range maximum; every wrong-argument and wrong-arity row with its path; the function-named-`UUID` ambiguity with code and two loci; lowercase `timestamp`/`uuid` parameters; every FR-056-AC-41 mismatch and `operator-ineligible` row; lowering to `value`/`literal` nodes of each kind and value, `Timestamp(-0)`/`Timestamp(0)` sharing one `"0"` node, two `"abc"` sharing one node, and the literal as the second argument of the native equality with empty laws and leaves; every FR-056-AC-40 evaluation vector, including call, `let` and field-read positions; and the TC-1052 step 6 replay rows with settlement and disagreement cause.
