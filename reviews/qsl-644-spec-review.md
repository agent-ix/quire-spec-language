---
id: SR-1352
title: "Spec review of quire-spec-language PR #649: FR-060 compile entry, TC-908, and the optional-field spelling fix (QSL-644)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@3d53d1dd4c57afaac44a9112bc8ae1a270fa969f; PR #649 diff against origin/main: spec/functional/FR-060, FR-092, FR-093, spec/test-cases/TC-908, TC-413, spec/spec.md, spec/tests.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-060
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-908
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-092
    type: reviews
---
# Spec review of quire-spec-language PR #649

## Summary

Ticket: QSL-644. Base spec review. The diff's scope covers integrity (AC to TC trace, index rows) and EARS conformance of the new statement.

- FR-060 "The facade's compile entry" is one SHALL statement and is testable. It names the signature, the return value, the shared spine run and the shared refusals, and it agrees with the code. AC-5 to AC-7 map onto TC-908 steps 1 to 5. The tests.md row lists all three ACs. `quire validate` on FR-060, FR-092, FR-093, TC-413 and TC-908 reports no errors.
- The `next: X?` fix is correct. The grammar's `P::Field` is `ident ":" TypeReference "?"? ";"` (qsl-cst/src/grammar.rs:437-446). `qsl-forms` maps the trailing `?` to `Presence::Optional` (qsl-forms/src/value.rs:414). The G2 golden test builds `next` with `Presence::Optional` (qsl-semantics/src/check/lowering/tests.rs:624-645). So `next: List?` is the optional field G2's key `8a69ece8...` was minted from, and the key needs no change.
- TC-202's validate failure was there before this PR and is not a finding.

Examined:
- FR-060 "The facade's compile entry" statement (examined)
- FR-060-AC-5, FR-060-AC-6, FR-060-AC-7 (examined)
- TC-908 (examined)
- FR-092 G2 rows and the Recursion-group vector list (examined)
- FR-092-AC-9, FR-092 D3 and the declared-record key-rule row (examined)
- FR-093 Recursive text-leaf vectors and FR-093-AC-11 (examined)
- TC-413 steps 7 and 9 (examined)
- spec/spec.md FR-060 row, spec/tests.md TC-908 row (examined)
- FR-060 Status, ADR-011 qsl-replay crate row (context_only)
- FR-262-AC-2, TC-735 (context_only)

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The optional-field respelling stops at `next`. The same files still spell optional fields in the old `f?: T` form, which no longer parses. FR-092-AC-9 and vector D3 say `record Opt { a: Int[0, 9]; b?: Int[0, 9]; }`. FR-092's key-rule row defines the optional-field key over "`f?: T`". FR-093's mutually recursive pair (`b?: B`, `a?: A`, lines 449-450), its leaf rule (lines 259 and 275) and FR-093-AC-11 (`record R { t?: Text[0, 64; nfc]; }`) do the same. Outside the diff, so do FR-262-AC-2 (`tail?: List`) and TC-735. Each of these is an acceptance criterion or golden vector whose source text the compiler refuses. Respell every one as `f: T?` in this PR. | spec/functional/FR-092-key-type-parameter-and-declared-nodes.md:936, spec/functional/FR-092-key-type-parameter-and-declared-nodes.md:283, spec/functional/FR-092-key-type-parameter-and-declared-nodes.md:407, spec/functional/FR-092-key-type-parameter-and-declared-nodes.md:556, spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md:449-450, spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md:704, spec/functional/FR-262-evaluate-values-and-calls-at-any-depth.md:54, spec/test-cases/TC-735-deep-values-and-value-types-evaluate-key-and-drop.md:22 |
| FND-002 | low | FR-060 gains a facade requirement, but its Status section still describes only the `arch-lint api-surface` check, and ADR-011's `qsl-replay` crate row still says the crate's entry is `qsl_replay::replay`. A reader looking for the facade's public API finds no mention of `compile_package` in either place. Add one Status line ("the compile entry is implemented in qsl-replay/src/compile.rs, TC-908") and name `compile_package` in the ADR-011 row. | spec/functional/FR-060-check-qsl-api-surface-boundary.md:254-283 |
| FND-003 | low | TC-908 says the test runs "From an integration test that reaches only the `qsl_replay` root". Its oracle, however, reaches `qsl_replay::spine`, `qsl_semantics::model::intake::package_input` and `quire_exact::Cancel`. Only the calls under test are root-only. Reword to "calls `compile_package` only through the `qsl_replay` root, and runs the spine directly as the oracle". | spec/test-cases/TC-908-the-facade-compiles-source-to-checked-package-bytes.md:20 |
| FND-004 | low | The tests.md TC-908 row's status is "🚧 In review", which no other row uses. The test passes at this head. Use "✅ Passed locally", as the other qsl-replay rows do. | spec/tests.md:537 |

## Verdict

One medium and three low findings. FND-001: the `f?: T` to `f: T?` respelling is incomplete, and two ACs (FR-092-AC-9, FR-093-AC-11) and several vectors still spell source that does not parse. The new FR-060 statement and AC-5 to AC-7 are sound, testable and backed. Not mergeable until FND-001 is fixed. Fix the low findings in the same round.

## Dispositions

Round 1, reviewed at 723980bb278a024eec9dc3dfdf86901ca13c6373.

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 723980bb |
| FND-002 | fixed | 723980bb |
| FND-003 | fixed | 723980bb |
| FND-004 | fixed | 723980bb |
