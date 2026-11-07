---
id: SR-20261107178701
title: "Base review of the extension selection, clause checking and lock rows change"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language branch spec/qsl-ext-admission, diff origin/main...HEAD: FR-110 (Extension selection, Lock rows, CON-1, AC-14, AC-15), FR-354 (Compile path, Clause checking, Reading, refusal rows, AC-2, AC-5, AC-7 to AC-9), TC-490 steps 13 and 14, TC-882 steps 2, 4 and 5, TC-883 step 7, TC-1308 row 20; read against qsl-cst TypeReference, qsl-package emit catalog_selections, qsl-replay spine admission_evidence, qsl-package read_import_view and read_evaluation_package, Contract IR ExpectedType::from_text, QSpec FR-469 and FR-513-AC-10"
review_set: base
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-110
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-354
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-490
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-882
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-883
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-1308
    type: reviews
---

## Summary

Tickets QSL-1787 and QSL-1583. This review covers only the committed diff
`origin/main...HEAD` of branch `spec/qsl-ext-admission`. It uses the base
checklist and runs no analysis lenses.

What holds:

- **Code names.** `ProfileCatalogInput` (`qsl-replay/src/spine/lifecycle.rs`),
  `DefinitionCatalog` and `DefinitionRole` (`qsl-semantics/src/library/bundle.rs`),
  `DefinitionCatalog::hold_extensions(selected, core)`,
  `complete::parse_with_extensions`, `Emission::evidence`,
  `qsl_package::read_import_view`, `qsl_package::read_evaluation_package`,
  `qsl_replay::spine::admission_evidence` (`qsl-replay/src/spine/bind.rs`,
  which clones the emission's evidence and extends it) and
  `DeclarationCause::Recursion` (`ill_typed`/`type-mismatch`) all exist as
  cited. `DefinitionRole::Extension` does not exist yet; the spec states the
  target design.
- **Lock rows.** The order (closed rows, then laws, then extensions, each
  once) matches `catalog_selections` in `qsl-package/src/emit.rs`, which
  writes the edition separately and deduplicates. The new extension tail is
  the only addition.
- **TC-1308 row 20** now names `s3.input_bytes` and `i2.input_bytes`, as
  FR-513-AC-10 does and as `qsl-package/src/emit/tests/temporal_clause.rs`
  tests.
- **IR agreement.** The v1 scalar set and `Int[lo, hi]` are a subset of
  Contract IR's `ExpectedType::from_text`, which also reads record names and
  extension node kinds. IR also accepts a bare `Int`, which QSL refuses; the
  subset is safe. `Int[0, 9; floor]` fails IR's bound split, so refusing it
  at S3 agrees.
- **Coverage.** Each new AC has a TC step: FR-110-AC-14 and AC-15 in TC-490
  steps 13 and 14, FR-354-AC-7 and AC-8 in TC-882 steps 4 and 5, and
  FR-354-AC-9 in TC-883 step 7. FR-469 states the form/record
  `ambiguous-name` collision.
- **Ceremony.** No digest, pin, SHA or tracking record was added.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | The S1 core `TypeReference` production cannot read the v1 scalar set as specified. In the complete grammar `Text`, `Rational` and `Decimal` are reserved keywords, and `TypeReference` admits them only with brackets: `Text[lo, hi; profile]`, `Rational[..; ..]` and `Decimal[..; ..; mode]`. So bare `Text`, `Rational` and `Decimal` do not parse, and by the bullet's own rule they "SHALL refuse" at S3. That makes the canonical `subject: Text` clause of FR-354-AC-1 and AC-6 refuse, and it contradicts AC-7's `t: Text`. `String`, `UUID` and `Timestamp` are not keywords. They parse only through the `QualifiedName` alternative, the same alternative a record-type name or a node kind takes, so the spec does not say how they are told apart from a record named `String`. The example `Option<Text>` does not parse at all (inner `Text` needs bounds); it does not "parse to another alternative". Fix: define the reader in two steps. (1) A spelling that is exactly one of the ten scalar words is that scalar, matched lexically as IR's `from_text` matches it. (2) Any other spelling is parsed with `TypeReference`, and only the `Int[lo, hi]` alternative (no rounding mode) and the `QualifiedName` alternative (a record name, or a held node kind) are supported. State that a scalar word is never looked up as a record name. Correct the `Option<Text>` example, or use one that does parse to another alternative, such as `Sequence<Integer>`. | spec/functional/FR-354-admit-extensions-from-declarative-grammar-and-typed-node-schemas.md:149-160,259,265; qsl-cst/src/grammar.rs:435-516; qsl-cst/src/token.rs:191-193 | wrong-requirement |
| FND-002 | medium | "Extension selection" gives two classifiers and calls them equivalent: role `DefinitionRole::Extension`, and a `Definition::is_extension()` that is true when the exact bytes hold `grammar_schema` or `typed_node_schema`. They disagree on a `Source`-role definition whose bytes hold a `grammar_schema`, and on an `Extension`-role definition that holds neither member. The second case must still reach `missing-member`, so it cannot be refused at E3. Two readers implementing the two branches classify these definitions differently. Fix: classify by role alone, and delete the "equivalently" parenthetical. A role-`Extension` definition that is missing a member already reaches FR-354's `missing-member` in `hold_extensions`. If `is_extension()` is kept, define it as `role == DefinitionRole::Extension`. | spec/functional/FR-110-resolve-header-profile-selections-at-e3.md:118-127 | wrong-requirement |
| FND-003 | medium | The bullet "An identity the catalog holds under any other role ... falls to the `unsupported-selection` row" conflicts with the table's `wrong-selection-role` row. That row covers an identity that is a qualification definition's and no layer's. If the supplied catalog holds a qualification identity (such as the `ieee_profile` row's) under a non-`Extension` role, the bullet says `unsupported-selection` and the table says `wrong-selection-role`. Fix: write "falls to the table's remaining rows (`unsupported-selection`, or `wrong-selection-role` for a qualification definition's identity)". | spec/functional/FR-110-resolve-header-profile-selections-at-e3.md:95,132-134 | wrong-requirement |
| FND-004 | medium | FR-354-AC-5 now reads the package with Contract IR's `read_checked_package` under the evidence that `qsl_replay::spine::admission_evidence` builds. TC-884 was not updated: its description and steps 3 and 4 still say "QSL's I2 reader" with an `evidence` argument. The AC's negative case is also not constructible as written. `admission_evidence` starts from `emitted.evidence()`, and "Reading" requires the emission's evidence to hold every named extension definition, so "that evidence built without the `retention` definition" cannot come from that function. Fix: state how the test builds the withholding evidence, for example the same `supply_closure` path over a `CheckedPackageEvidence` into which the extension definition was not inserted. Update TC-884 description and steps 3 and 4 to name `read_checked_package`, `admission_evidence` and that construction. | spec/functional/FR-354-admit-extensions-from-declarative-grammar-and-typed-node-schemas.md:263; spec/test-cases/TC-884-extension-admission-leaves-other-units-and-backends-alone.md:13,21-22 | correct-requirement-no-evidence |
| FND-005 | low | The new trailing-separator bullet names the `}`, `clause-name` and the expected clause, but not the code and cause. The refusal-table row for "with a trailing separator" still lists only "the actual token and the expected category", without the clause. AC-2 gives `invalid_syntax`/`unexpected-token`. Fix: write the code and cause into the bullet, and add "and, for a trailing separator, the expected clause" to the table row. | spec/functional/FR-354-admit-extensions-from-declarative-grammar-and-typed-node-schemas.md:141-143,237 | wrong-requirement |
| FND-006 | low | The unsupported-spelling refusal is a `type-mismatch` that names "the unreadable spelling as the expected type", but states no actual type. Every other `type-mismatch` row names both an expected and an actual type. Fix: state the actual (the clause value's checked type), or state that the refusal carries no actual type, in both the bullet and the table row. | spec/functional/FR-354-admit-extensions-from-declarative-grammar-and-typed-node-schemas.md:158-160,240 | wrong-requirement |
| FND-007 | low | "Lock rows" item 1 does not order the closed rows within the group. `catalog_selections` writes them "in the lock's row order, and no layer that requires a selected layer". It also does not say whether two header profiles that name one extension yield one selection or a refusal. E3 returns selections "in header source order", and the lock says only "each once". Fix: add "in the lock's row order" to item 1, and state the duplicate-selection outcome. | spec/functional/FR-110-resolve-header-profile-selections-at-e3.md:129-131,251-265; qsl-package/src/emit.rs:2596-2607 | wrong-requirement |
| FND-008 | low | FR-354-AC-7 and TC-882 step 4 say "a `Window` value" without giving its spelling or `Window`'s members, so the vector is not reproducible. The node-kind cycle rule ("reach a form again") has no self-reference case (`link A { to: A }`). Fix: give `record Window { lo: Integer; }` and the clause value's exact source text in AC-7 and step 4. Add `link A { to: A }` to AC-8 and step 5. | spec/functional/FR-354-admit-extensions-from-declarative-grammar-and-typed-node-schemas.md:177-180,265-266; spec/test-cases/TC-882-an-extension-form-parses-checks-and-packages-as-a-typed-node.md:25-26 | correct-requirement-no-evidence |

## Verdict

The planner's rulings are carried through for extension selection, lock
rows, node-kind clauses, the collision, out-of-range integers, the Reading
section's code names and TC-1308 row 20. FND-001 is a contradiction: the
named reader cannot read three of the ten v1 scalars, and that refuses the
AC-1 example. Fix it in this PR, together with FND-002 to FND-004.
FND-005 to FND-008 are low-severity clarifications.
