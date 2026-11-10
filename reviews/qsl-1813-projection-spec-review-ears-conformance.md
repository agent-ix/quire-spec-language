---
id: SR-20261109172912000103
title: "QSL-1813 source-refusal projection spec-review/ears-conformance review"
type: SpecReview
analysis: ears-conformance
review_set: subset
scope: "agent-ix/quire-spec-language; QSL-1813; PR #2632; FR-001 and TC-424"
relationships:
  - { target: ix://agent-ix/quire-spec-language/FR-001, type: references }
  - { target: ix://agent-ix/quire-spec-language/TC-424, type: references }
---
## Summary

Independent spec-review/ears-conformance review of frozen head 2cf75c67957e4303120a18aeed66b052bad3bfed. No in-scope defect found.

## Assurance Context

No AssuranceProfile document exists under spec/. The lead selected direct base, EARS, integrity and dependency analyses. No enforced run or human acceptance is claimed.

## Verdict

PASS for specification readiness after review artifact custody. This does not qualify implementation, extend wire members, close FR-287-AC-3 or complete QSL-1813.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Analysis

The added event requirement names a supplied typed refusal as trigger, CST as subject, and public projection as measurable response. Retention and limit obligations are concrete and independently asserted by the ACs. Scoped repository validation reports no grammar warning for either changed document; its 28 advisory findings are in unchanged documents. No trigger-state confusion or ambiguous failure obligation was found.

## Verification and Limits

Repository spec validation succeeds; 28 existing advisory grammar findings affect unchanged documents, with none in FR-001 or TC-424. Diff check and frozen-head checks pass. No edited document is embedded by a Rust include. No Cargo, Kani, runtime test, full matrix, implementation qualification, source edit or push was performed.

## Reviewed Units

```json
{
  "clean": true,
  "scope": [
    {
      "id": "FR-001-AC-13",
      "path": "spec/functional/FR-001-read-exact-source.md",
      "role": "examined",
      "excerpt": "A caller outside CST constructs a typed blank-label refusal using `first_blank_label` and the existing public refusal fields, then calls `source_refusal` without bytes or an admitted source. For empty, space-only and U+3000 labels, including both labels blank, the projected identity cause retains the first blank label in authority-then-identity order; U+200B is not blank. An empty-path refusal retains `empty-path` with no label."
    },
    {
      "id": "FR-001-AC-14",
      "path": "spec/functional/FR-001-read-exact-source.md",
      "role": "examined",
      "excerpt": "For every `SourceReadCause`, public projection gives the code and complete cause in the mapping above, retains source identity, path, message and optional region exactly, sets phase `Source` and has no related regions. Cases with no region retain `None`; located UTF-8, BOM and NUL cases retain their original reference and endpoints."
    },
    {
      "id": "FR-001-AC-15",
      "path": "spec/functional/FR-001-read-exact-source.md",
      "role": "examined",
      "excerpt": "Projecting `ByteBudget` under ceilings below and above 1 MiB retains exactly the supplied bound as `SyntaxLimit::SourceBytes`; projecting every other cause under either ceiling has no syntax limit. Source refusals returned through `read_source` have the same projected fields as direct public projection of the corresponding `Source::read_typed` refusal with the same ceiling."
    },
    {
      "id": "TC-424",
      "path": "spec/test-cases/TC-424-a-read-source-carries-its-caller-named-source-reference.md",
      "role": "examined",
      "excerpt": "Scope: FR-001-AC-5 to FR-001-AC-15."
    },
    {
      "id": "read_source",
      "path": "qsl-cst/src/diagnostic.rs",
      "role": "context_only",
      "excerpt": "Source::read_typed(identity, path, bytes, byte_limit).map_err(|refusal| {"
    },
    {
      "id": "SourceReadRefusal",
      "path": "qsl-foundation/src/source.rs",
      "role": "context_only",
      "excerpt": "pub struct SourceReadRefusal {"
    },
    {
      "id": "FR-001",
      "path": "spec/functional/FR-001-read-exact-source.md",
      "role": "context_only",
      "excerpt": "The path remains a locator; the projection does not derive source identity\nfrom it or mint a source reference."
    }
  ],
  "findings": []
}
```
