---
id: SR-957
title: "QSL-358 slice 2 spec review of PR 566: ADR-011 SV row, §6.1 K-leaf rule, §6.2 module rows and X-11"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@bcc946cd8154ceefeb1ee0f9cb476d359b726015; spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md (diff a7df1ff0...bcc946cd: lines 715, 776-780, 901-902, deleted value::containment row, 1099); context: SV bullet 781-796, FB-05 572, ADR-012 1550/1554, ADR-016 113-117; overlap with PR 565's open fix round (SR-954)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: reviews
---
## Summary

Ticket: QSL-358 (slice 2). PR: quire-spec-language#566 at bcc946cd.

Edits examined: the §6.1 SV row (715), the "K is a leaf" rule sentence (776-780),
the §6.2 SV move row (901), the §6.2 layer-3 `value` row (902), the deleted
`value::containment` row, and X-11 (1099). Each now places `declaration`,
`containment` and the enum runtime half in SV. That matches the crate, and the
deleted `value::containment` row was stale, so deleting it is correct. The old
layer-3 row claimed `declaration` "imports only K, SV and ... `enumeration`".
That was false, because it imported `model::domain_package`, so dropping it is
correct too.

Overlap with slice 1's open fix round (SR-954 FND-001, FND-002, FND-005 and
FND-006). That round rewrites the same lines: the SV row's dependency cell and
module list, §6.1 and X-11 to add `semantic_node`, and X-11's "Extracted in
part". It is not pushed yet (`origin/task/358-s1-quire-semantic-value` is still
a7df1ff0). Slice 2's lines 715, 776-780, 901 and 1099 will all conflict when
slice 2 rebases. FND-001 to FND-003 say what the merged text has to keep.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The rewritten §6.1 SV row adds `enumeration`, `declaration` and `containment`, but it still leaves out `semantic_node`, which the crate exposes. Its "Depends on" cell still reads "K; `quire-canonical` without `std` for the compound-unit id", while X-11 in the same diff lists `quire-exact`, `quire-canonical`, `serde` and `thiserror`. The `declaration` module now uses `thiserror` derives too. Slice 1's fix round rewrites this exact line (SR-954 FND-001/FND-002), so the rebase conflicts. The merged row must list `stop`, `quantity`, `unit`'s runtime half, `semantic_node`, `enumeration`'s runtime half, `declaration` and `containment`, and use slice 1's corrected dependency text. | spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md:715 |
| FND-002 | medium | X-11 keeps "**Extracted** in part" in the line this PR rewrites. That is forward-looking: it implies more moves without recording them. Slice 1's fix round deletes it (SR-954 FND-005). The module path list `quire_semantic_value::{stop, quantity, unit, enumeration, declaration, containment}` also leaves out `semantic_node`, as does the §6.2 move row (901). After the rebase, say "Extracted" and list every module the crate holds, `semantic_node` included, in both X-11 and row 901. | spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md:901,1099 |
| FND-003 | medium | The ADR gives no layer for the two new layer-3 modules. Row 901 says operations "stay at layer 3 in `value::operation`" and names `value::environment_stage`, but neither has a §6.2 module row, and the layer-3 `value` row (902) names neither. `value::operation` imports `model::domain_package::OperationEffect`, and `model::observation` imports `value::operation`. As a `semantic_value` module that is an upward import against §6.1's `semantic_value < model`. As a `model` module it matches `value::model_query`, which the table already maps to 3 `model`. Give `value::operation` a row at 3 `model`, or move it into `model` (SR-955 FND-002) and name it there. Put `value::environment_stage` (F + SV imports only) in the 3 `semantic_value` row. | spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md:901-902,912 |
| FND-004 | low | The rewritten "K is a leaf" sentence lists the `Stop` carrier, quantities, the unit graph's runtime half, the runtime enum values and the registry, but not the `semantic_node` refusal vocabulary. Slice 1's fix round adds `semantic_node` to this same sentence, so it will conflict. Keep both sets of additions when merging. | spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md:776-780 |
| FND-005 | low | This PR leaves the SV bullet incomplete. The bullet says "SV mints no kernel identity" and "The one digest it computes is a compound unit's ... `UnitId`". SV now also holds `EnumValue::variant`, which calls the kernel's `VariantId::from_digest` on a member key, and the public trusted `EnumValue::admitted`. That is a retype, not a mint, but the bullet should say SV retypes an admitted enum member key as its `VariantId` and computes nothing for it. It can drop that sentence once slice 3 removes `admitted`. | spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md:788-794 |
| FND-006 | low | Other ADRs cite the deleted file by line. ADR-012 cites `qsl-semantics/src/value/declaration.rs:1160` and `:1055-1060` (`check_recursion`, `type_refusal`). ADR-016 cites `value/declaration.rs:884` and `:966`. None of those paths exist after this PR. Line citations into code go stale on every edit. Name the item (`quire_semantic_value::declaration::TypeEnvironment::check_recursion`) and drop the path and line. | spec/decisions/ADR-012-semantic-family-extension-contracts.md:1550,1554; spec/decisions/ADR-016-state-model-finite-execution-mapping.md:113-117 |

## Verdict

The ADR edits are correct about where the code now lives, and deleting the stale
`value::containment` row and the false "imports only" claim is right. The edits
collide with slice 1's open fix round on four lines. FND-001 and FND-002 say what
the merged text has to keep. FND-003 is a real gap: the new modules have no
layer, and one of them imports upward. Fix FND-001 to FND-003 in this PR after
#565's fix round lands and slice 2 rebases. FND-004 to FND-006 are low.
