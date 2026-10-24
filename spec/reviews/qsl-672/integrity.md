---
id: SR-5668
title: QSL-672 PR 731 integrity review
type: SpecReview
analysis: integrity
scope: agent-ix/quire-spec-language@bef47c6353ef00207df211d79fe4c9304bc3ae23; spec/functional/FR-091-produce-value-forms-and-assemble-package-declarations.md, spec/test-cases/TC-394-value-expression-mapping-and-precedence.md, spec/test-cases/TC-396-s2-refuses-unrepresented-and-foreign-constructs.md
review_set: subset
---

## Summary

Ticket: QSL-672. The revised S2/check boundary, law ownership, OQ-5 split, and complete-value-only slash promotion agree with QSpec FR-147, FR-149, FR-044 and shared grammar at 9b709948c39ecc40210589ba5ec418ef5a81c295. The new test vector does not discriminate two of the required laws.

## Verdict

**CONDITIONAL**. Resolve the finding before a clean acceptance.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | TC-394 uses only (-7,3) to distinguish truncating, floor and Euclidean div/rem. Floor and Euclidean both yield q=-3,r=2, so a floor implementation substituted for Euclidean passes; mod inheriting floor also returns the required 2. Add a negative-divisor vector such as (7,-3): floor q=-3,r=-2 versus Euclidean q=-2,r=1, with mod=1 under every selected law. | spec/test-cases/TC-394-value-expression-mapping-and-precedence.md:40 |

## Examined scope

```yaml
scope:
- id: FR-091-AC-7
  path: spec/functional/FR-091-produce-value-forms-and-assemble-package-declarations.md
  role: examined
  excerpt: 'S2 refuses the whole unit with cause `UnrepresentedConstruct`, naming the construct''s CST production and span and returning no form, for a function body that holds exactly one of: a text literal, `decimal(1, 2)`, `xs[0]`, `none`, `reaches(a, b, M::R)`.'
- id: FR-091-AC-39
  path: spec/functional/FR-091-produce-value-forms-and-assemble-package-declarations.md
  role: examined
  excerpt: S2 maps `a div b`, `a rem b` and `a mod b` to distinct `IntegerDiv`, `IntegerRem` and `IntegerMod` binary forms with operand order and original spans, including in an unreachable branch. Check admits each well-typed operator under a resolved `quire.value.complete/v1` selection, using the checked package's exactly-one law for `div`/`rem` and Euclidean meaning for `mod`; without that facet, each receives a located `unsupported_construct` rather than `UnrepresentedConstruct`. `/` never becomes integer division; only under complete value does a rational `/` with integer operands and an expected rational type follow QSpec FR-149.
- id: FR-091
  path: spec/functional/FR-091-produce-value-forms-and-assemble-package-declarations.md
  role: examined
  excerpt: S2 preserves `div`, `rem` and `mod` as distinct operator forms with their CST
- id: FR-091
  path: spec/functional/FR-091-produce-value-forms-and-assemble-package-declarations.md
  role: examined
  excerpt: '| FR-091-OQ-5, integer operators | Resolved: S2 maps `div`, `rem` and `mod` to distinct forms regardless of reachability. Check uses the declaration''s resolved `using` selection: absent `quire.value.complete/v1` refuses each at its operator locus; selected complete value admits well-typed forms. The checked package''s exactly-one integer-division law governs `div`/`rem`; `mod` stays Euclidean. Their checker and evaluator arms are required by FR-091-AC-39 and QSpec FR-147. `/` has no integer-division meaning; QSpec FR-149 owns exact integer-to-rational promotion only under complete value. | QSpec FR-147 and its shared grammar admit these operators only under the complete-value facet, while QSpec FR-039 preserves the state-only refusal. S2 must not discard the distinction before profile admission. | QSpec changes the facet, operator law or slash rule. |'
- id: FR-091
  path: spec/functional/FR-091-produce-value-forms-and-assemble-package-declarations.md
  role: examined
  excerpt: '| FR-091-OQ-5, other constructs | `collect` maps to `Query{Map}`. `reaches` is a `StateModel` construct with no variant; it and the other listed unrepresented constructs are refused with `UnrepresentedConstruct`. A variant for one of them comes with its checker and evaluator arms. Float literals depend on a mode-carrying `ValueType::Float` (FR-091-OQ-4), and `e[i]` on QSpec semantics. | QSpec FR-145 makes `map` and `collect` one operation. QSL FR-008-AC-20''s duplicate-output `collect` refusal belongs to the native lane, not to complete-V1; QSpec FR-008-AC-5 refuses `collect` only where `quire.state.queries/v1` is selected without `quire.value.complete/v1`. | M-6a must compile existing native programs that use the refused constructs. |'
- id: TC-394
  path: spec/test-cases/TC-394-value-expression-mapping-and-precedence.md
  role: examined
  excerpt: "Every fixture unit below starts with the complete-V1 header\n(`language \"ix:native\" edition \"1-draft\";`) and one profile selection whose\nalias is `v`.\n\n1. For each row of FR-091's expression-mapping table, build an admissible\n   unit whose one function body holds that construct over names `a`, `b`,\n   `c`, as a table-driven test with one case per row.\n2. Run S2 and read the function's body.\n3. Separately run S2 on the bodies `a or b and c`, `a - b - c`,\n   `a implies b implies c` and `(a + b) * c`.\n4. Run S2 on the bodies `map(x in c: x)`, `collect(x in c: x)` and\n   `allInstances<M::T>(p)`.\n5. Run S2 on `a div b`, `a rem b`, `a mod b`, `1 + a div b * 2` and\n   `if true then 1 else a mod b`, retaining each operator and operand span.\n   Check each well-typed body under a resolved `quire.value.complete/v1`\n   selection with exactly one of the truncating, floor or Euclidean\n   integer-division definitions, and separately check `mod` without selecting\n   a division law when no `div`/`rem` occurs. Use `(-7,3)` to distinguish\n   the selected `div`/`rem` law and require `mod` to expose `2` under every\n   selection. Check `7 / 3` with integer operands both without an expected\n   rational type and with expected `Rational[0,7;1,3]`, under both state-only\n   and complete-value selections.\n\nTag the test `#[trace(\"FR-091-AC-3\", \"FR-091-AC-39\")]`."
- id: TC-396
  path: spec/test-cases/TC-396-s2-refuses-unrepresented-and-foreign-constructs.md
  role: examined
  excerpt: "Every fixture unit below starts with the complete-V1 header\n(`language \"ix:native\" edition \"1-draft\";`) and one profile selection whose\nalias is `v`.\n\n1. For each body below, build an admissible unit whose one function body is\n   that construct, and run S2: `\"text\"`; `decimal(1, 2)`;\n   `xs[0]`; `none`; `reaches(a, b, M::R)`.\n2. Run S2 on `a div b`, `a rem b`, `a mod b` and\n   `if true then 1 else a mod b`, each under a state-only declaration\n   without `quire.value.complete/v1`; pass the resulting forms to check.\n3. For each body `B` of `deref(x)`, `allInstances<M::T>(x)` and `pre(x)`,\n   build an admissible unit, with no admitted domain package, whose one\n   declaration is `function f using v(x: Int[0, 9]): Boolean pure { B }`.\n   Do the same for\n   `function g using v(x: Int[0, 9]): Boolean pure decreases(pre(x)) { true }`.\n   Run S2, then the assembler, then `PackageDeclarations::check` on\n   whatever the assembler returns.\n\nTag the test `#[trace(\"FR-091-AC-7\", \"FR-091-AC-8\", \"FR-091-AC-39\")]`."
```

## Evidence and limits

PR head and body (`Refs QSL-672`) independently verified. Changed-document `quire validate` exited 0; author parse/lint logs report 3/3 completed with advisory module warnings. Full author validation reports 1542/1550 grammar-clean and 17 grammar advisories outside the changed documents. No applicable AssuranceProfile was found. No builds, repository edits, tracker posts or merge were performed, per the read-only dispatch. Runnable implementation coverage is separate from this specification review.

The optional repository-wide `quire matrix` scan produced no rows before the worktree moved to a new head and was cancelled (exit 130). No matrix result is claimed. The authored trace examples were inspected directly; a TC document annotation alone is not a runnable test binding.

## Dispositions

Re-reviewed at c3c65a7b0cd72f76f8c51967d88027c2884699d9. **PASS**; no new findings. Original finding text and review scope are retained above.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed c3c65a7b0cd72f76f8c51967d88027c2884699d9 | TC-394 now supplies all four sign quadrants for each law. All 12 quotient/remainder cells independently match integer arithmetic; Euclidean mod is 1, 2, 1, 2 under every selection. |

The whole added-line scan confirms both changed AC Verification cells are exactly `Test` and both trace examples contain only AC IDs. Prior semantic admission, OQ-5 and FR-044/FR-149 promotion conclusions remain valid. Remote PR head matches the reviewed head; body is `Refs QSL-672`. Author validation is 3/3 grammar-clean; parse/lint completed 3/3 with existing advisory module warnings. No builds or repository edits were performed.
