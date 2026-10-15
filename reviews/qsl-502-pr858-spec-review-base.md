---
id: SR-11181
title: "base review of quire-spec-language PR 858 (union duplicate-member code and producible member-ambiguity vectors)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@82d3c2c43b97f328644f4d43537317e0afb59815; spec/functional/FR-316-check-union-declarations.md (Behavior, AC-3), spec/functional/FR-317-resolve-member-references-and-check-union-construction.md (AC-3), spec/decisions/ADR-012-semantic-family-extension-contracts.md (§16.5 declaration row, §16.12 adverse-declaration row), spec/test-cases/TC-818-s3-refuses-ill-formed-union-declarations-and-bounds-them-only-by-check.md (step 3), spec/test-cases/TC-820-s3-resolves-qualified-member-names-by-candidate-and-refuses-ambiguity.md"
review_set: subset
---

## Summary

Ticket: QSL-502. Base checklist over the five edited units, with every code
claim checked against `origin/main`.

- `AssemblyCause::code()` maps `DeclarationCause::DuplicateMember` to
  `Code::AmbiguousDeclaration` and `catalog_code()` maps it to
  `ambiguous-name` (`qsl-semantics/src/check/assemble.rs:408`, `:486`). The
  FR-316 and ADR-012 rows now match the code.
- The assembler refuses every declaration after the first that reuses a
  declared type name with `AmbiguousTypeName`, at that declaration's span
  (`assemble.rs:2213-2226`). The sibling union-check branch registers unions
  through the same `unit.declare` path, so a union and an enum, or two
  unions, named `q` are refused there.
- An import contributes only its alias. `ImportedTypeName` refuses a type
  position that names an imported declaration (`assemble.rs:296-303`,
  `:1568-1577`). `DuplicateAlias` covers profile, import and model aliases
  only (`assemble.rs:2141-2170`), so no rule refuses a declared type `q`
  beside an import alias `q`. TC-820 step 1 can be built from source, and
  FR-317 Behavior gives the expected result: the candidates are the
  declared unions, declared enums and import aliases named `q` that have a
  member `m`, so two candidates refuse `ambiguous_declaration`/`ambiguous-name`.
- Hand-built scope steps 5-7: `ScopeIndex` keeps every type bound to a name
  and holds composites by their declaration name, not their key
  (`check/check.rs:720-735`). A crate-internal scope with two composites
  named `q` under distinct keys can be built, and the seam function is
  specified in ADR-012 §16.2 (`check::call_target`).
- Every AC sentence is bound to a TC step that can fail.
- No `.rs` file reads FR-316, FR-317, ADR-012, TC-818 or TC-820
  (`include_str!` only reaches FR-092, FR-093, FR-094 and FR-255).
- The diff adds no ticket ids, SHAs or compatibility wording.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | ADR-012 §16.2 (example "two unions named `q`") and the §16.12 "Adverse, resolution" row still present two unions or a union and an enum named `q` as plain S3 vectors. They do not say these cases exist only on hand-built scopes, as the rewritten FR-317-AC-3 and TC-820 now do. | spec/decisions/ADR-012-semantic-family-extension-contracts.md:1504, 1791 |

## Verdict

PASS. The code claims are correct on `origin/main`. The new source vector can
be built and FR-317's candidate rule makes its expected result right. Every
AC is bound to a step that can fail, and no test parses an edited file. One
low: stale wording in ADR-012.
