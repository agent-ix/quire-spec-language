---
id: SR-960
title: "QSL-358 slice 3 spec review of PR 567's ADR-011 edits, and overlap with slices 1 and 2"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@f0166db0e28e7cd401619ed93d6ea261cd895ae7; diff bcc946cd...f0166db0; spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md lines 715, 776-796, 901-904, 1099; overlap checked against slice 1 head a7df1ff0 (#565, SR-954) and slice 2 head bcc946cd (#566, SR-957)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: reviews
---
## Summary

Ticket: QSL-358 (slice 3). Slice 3 edits four ADR-011 rows: §6.1 SV (715),
§6.2 SV move row (901), §6.2 layer-3 `value` row (902) and X-11 (1099). Slice 2
already rewrote all four lines, and slice 1's open fix round (SR-954 FND-001,
FND-002, FND-005) rewrites 715, 901 and 1099 again. All three PRs edit the same
lines, so a rebase of slices 2 and 3 conflicts on every one of them. That is
expected and mechanical. The findings below are what slice 3's text gets wrong
or leaves contradictory. The merged text has to fix these too.

Correct in slice 3's edits: `EnumDeclaration` added to the SV `enumeration`
list. The definition refusal vocabulary is named in the SV row, row 901 and
X-11, with the right type list. Row 902 now limits layer-3 `definition` to "the
lock catalog and admission". `semantic_node` and `definition` are added to the
module path lists in 901 and X-11.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The SV row (715) now names `semantic_node`'s items as "`InvalidSemanticGraph`, `SemanticGraphCause`, `check_terms`" and calls them a refusal vocabulary. It leaves out `IDENTITY_LIMITS`, which the crate exposes and which the SV bullet (793) says lives in SV. `check_terms` is a validator, not vocabulary. The "Depends on" cell still reads "K; `quire-canonical` without `std` for the compound-unit id". The row now adds `definition`, whose `PackageRefusal` derives `thiserror::Error`, and X-11 lists `serde` and `thiserror`. Slice 1's fix round rewrites this cell and adds `semantic_node` to this row (SR-954 FND-001/FND-002), so this line conflicts three ways. Merged text: `semantic_node` (the refusal vocabulary, `check_terms` and `IDENTITY_LIMITS`), and slice 1's corrected dependency list. | spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md:715 |
| FND-002 | medium | X-11 (1099) keeps "**Extracted** in part" in the line this PR rewrites. That is forward-looking text. Slice 1's fix round deletes it (SR-954 FND-005), and SR-957 FND-002 asks slice 2 to do the same, so slice 3 contradicts the agreed text. Say "Extracted" and list the modules the crate holds. The list slice 3 gives (`stop, quantity, unit, enumeration, semantic_node, definition, declaration, containment`) is complete. | spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md:1099 |
| FND-003 | low | `semantic_node` now has two §6.2 rows that disagree. Row 902 lists "the compile-side halves of `enumeration`, `semantic_node` and `unit`" at 3 `semantic_value`. Row 904 (`value::semantic_node`, 3 `semantic_value`, "the I04 node-identity preimage machinery") was not touched and still reads as the whole module. Row 901 also adds `value::semantic_node`'s vocabulary, which slice 1's fix round adds to the same row, so the semantic_node text would be added twice. Drop `semantic_node` from 902 and make 904 say "compile side: owner projection, JCS digest, `WireNodeId` reading; the refusal vocabulary is in SV". When merging, keep one `semantic_node` clause in 901. | spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md:901,902,904 |
| FND-004 | low | Two §6.1 sentences this slice makes incomplete were left unedited. The "K is a leaf" sentence (776-780) lists what lives in SV but not the structural enum declaration or the definition refusal vocabulary. Slice 1's fix round adds `semantic_node` to the same sentence (SR-957 FND-004). The SV bullet (794-796) says "The compile-side half of a module whose runtime half is in SV stays at layer 3: for `unit` that is ...". It does not say what stays for `enumeration` (preimage reading, the node-key digests, the owner join, the stale-key and foreign-declaration checks). It also does not say that SV takes declaration and member keys as given and only retypes the member key as its `VariantId` (SR-958 FND-001, SR-957 FND-005). Add the enumeration clause and the key sentence. | spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md:776-780,788-796 |

## Verdict

Changes requested. Slice 3's own additions are accurate. The two mediums are
stale text that slice 3 left in lines it rewrote, and they conflict with slice
1's agreed fixes. The cleanest way to land: rebase slices 2 and 3 after slice
1's fix round merges, then apply FND-001 to FND-004 to the merged rows in one
pass.
