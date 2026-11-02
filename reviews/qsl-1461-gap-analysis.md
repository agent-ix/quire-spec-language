---
id: SR-202611021111
title: "Fresh gap analysis - QSL-1461 explicit Text literals"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; PR #2084 head ecb0d795b; FR-056-AC-51/52, FR-091-AC-69, FR-093-AC-50, TC-1052 steps 7-9"
review_set: subset
---

## Summary

Ticket: QSL-1461. Planless manual criteria-to-test check (Quoin matrix not run). Plan completion: not assessed. FR-056-AC-51/52 and FR-093-AC-50 (value half) have value-asserting tagged tests; FR-091-AC-69 has no tagged test; FR-093-AC-50's equality-lowering half is untested.

## Verdict

**NOT MERGEABLE** until FND-001 and FND-002 are closed.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-091-AC-69 is traced by no test. Its distinct vectors are unasserted: S2 builds a Call named `Text` with one Expression::String argument; `Text("\ud800")` refuses invalid_syntax/invalid-escape; a unit declaring `function Text` makes `Text("abc")` refuse ambiguous_declaration/ambiguous-name. Only the arity/argument/bounds refusals are covered, under other tags. | spec/functional/FR-091-produce-value-forms-and-assemble-package-declarations.md:1168 |
| FND-002 | medium | FR-093-AC-50 second half is unasserted: the equality `x = Text("é")` lowers to one text equality with law and mode `text_profile` nfc and arguments [ref(x), ref(literal)]; `Text("é")` and `Text("é")` lowering to the same single node; literal `semantic_type` equal to the body `type`. Tests check only the value set and that body type is a text_bounds node. | qsl-package/src/emit/tests.rs:2402; qsl-semantics/tests/it/native_literals.rs:355 |
| FND-003 | low | FR-056-AC-52 result positions Integer and UUID, and FR-056-AC-51 `Text[0, 2; binary-utf8]` admitting U+00E9, are not exercised by name (binary is covered only through vector TX-12/13). | qsl-semantics/tests/it/native_literals.rs:441 |

## Review Evidence

Covered with value assertions: AC-51 evaluation (replay compile_text_literals), r1 retained U+00E9, binary-utf8 two-byte refusal; AC-52 TextLiteralOutsideBounds payload (argument, min, max, profile, retained_length), call origin, empty path, String result and String peer, Text(x), Text(5), Text(), two arguments; AC-41 non-regression; FR-322-AC-304 vectors via emit and strict reader.
