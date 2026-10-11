---
id: SR-4970
title: "QSL-359 gap-analysis"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@8509dcc41150bc42f926981877b635fd63726fb4; relationship lowering; admission corpus; FR-093; FR-094"
review_set: subset
---

## Summary

QSL-359 / PR #730: CONDITIONAL. The diff selects relationship records and moves the corpus family row, but test compilation and production source-occurrence placement prevent acceptance; unconditional relationship provenance also lacks a scoped normative decision.

## Verdict

CONDITIONAL — findings require disposition.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The change roots every imported RelationshipRecord even when no checked node or frame names it | qsl-semantics/src/check/lowering/model.rs:360 |

## Evidence and scope

```yaml
{
  "clean": false,
  "scope": [
    {
      "id": "relationship declaration selection",
      "path": "qsl-semantics/src/check/lowering/model.rs",
      "role": "examined",
      "excerpt": "pub(crate) fn relationship_nodes(&mut self, location: &Location) -> Result<(), CheckRefusal> {"
    },
    {
      "id": "relationship lowering test",
      "path": "qsl-semantics/src/check/lowering/model/tests.rs",
      "role": "examined",
      "excerpt": "|lowering| lowering.relationship_nodes(),"
    },
    {
      "id": "production lowering hook",
      "path": "qsl-semantics/src/check/mod.rs",
      "role": "examined",
      "excerpt": "if let Err(refusal) = lowering.relationship_nodes(&relationship_location) {"
    },
    {
      "id": "admission corpus",
      "path": "qsl-package/src/emit/tests/admission_corpus.rs",
      "role": "examined",
      "excerpt": "K::Relation(RelationForm::Relationship),\n            F::RelationshipModel,"
    },
    {
      "id": "authored relationship fixture",
      "path": "tests/fixtures/spine-model-relationship.semantic-ir.json",
      "role": "examined",
      "excerpt": "\"identity\": \"ix://acme/orders/Widget\""
    },
    {
      "id": "occurrence completion",
      "path": "qsl-semantics/src/check/lowering.rs",
      "role": "examined",
      "excerpt": "let at = enclosing.get(key).unwrap_or(root);"
    },
    {
      "id": "region resolution",
      "path": "qsl-semantics/src/check/region.rs",
      "role": "examined",
      "excerpt": "Origin::TypeDeclaration { .. } | Origin::ProtocolAttempt { .. } | Origin::Expression => {\n            return None"
    },
    {
      "id": "emission source map",
      "path": "qsl-package/src/emit.rs",
      "role": "examined",
      "excerpt": "EmitRefusal::UnlocatedOccurrence {"
    },
    {
      "id": "FR-094",
      "path": "spec/functional/FR-094-key-model-owned-reference-population-and-quantity-nodes.md",
      "role": "examined",
      "excerpt": "`check` builds the node of each model declaration that a checked node or a\nframe subject (ADR-013 O-08) names:"
    },
    {
      "id": "FR-093",
      "path": "spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md",
      "role": "examined",
      "excerpt": "a node that no region denotes, such as the `Integer` and text scalar nodes\n  that type the literals inside a type or parameter node's body, has one\n  `generated` occurrence, so every node has at least one (FR-322)."
    }
  ],
  "findings": [
    {
      "fnd": "FND-001",
      "severity": "medium",
      "confidence": "high",
      "method": "gap-analysis",
      "check": "specification-gap",
      "artifact_id": "FR-094",
      "related_ids": [
        "QSL-359"
      ],
      "path": "qsl-semantics/src/check/lowering/model.rs",
      "lines": "360-376",
      "excerpt": "/// Key every relationship declaration selected by this unit.",
      "finding": "The change roots every imported RelationshipRecord even when no checked node or frame names it. FR-094 currently materializes named declarations only, and FR-093 anchors generated nodes through declarations that actually reach them. No spec change defines source identity/region or occurrence rules for an unreferenced imported relationship. QSL-359 already records this exact unresolved paired QSL/QSpec provenance decision (comment 11372528). Obtain and record that scoped normative ruling before treating unconditional emission or an arbitrary first-function placement as specified. Add cases for no functions, multiple/reordered functions, and multiple selected models against the chosen provenance rule. The new FR-094-AC-1 tag covers a key/vector obligation and does not itself define this new emission/provenance policy."
    }
  ],
  "bindings": [],
  "qualification": "Diff-scoped static Rust/code and acceptance review only; no production edits, agents, builds or aggregate matrix. git diff --check passed. No runtime gate claim. Plan completion: not assessed."
}
```

## Methods and limits

The Rust lane is folded into code-review. Gap assessment is restricted to changed behavior and its stated FR/TC owners, using source/spec comparison; no repository-wide computed matrix was requested or claimed. No specification text changed and no applicable AssuranceProfile was found. Plan completion: not assessed.

## Dispositions

Round 1, reviewed 29b81ae6d6bb07568e5fa250e3e27c4db5ac8a74. Current verdict: CONDITIONAL.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | still-open | The implementation now explicitly records a generated occurrence at a model alias declaration span. FR-093:92-101 and FR-094:127-131 remain unchanged, and the frozen delta contains no ruling making an unreferenced imported relationship a root or defining its model-selection source occurrence. The requested no-functions, reordered-functions and multiple-model provenance assertions are also absent. Record the paired scoped rule and verify those cases. |

## Delta evidence

```yaml
{
  "reviewed": "29b81ae6d6bb07568e5fa250e3e27c4db5ac8a74",
  "dispositions": [
    {
      "fnd": "FND-001",
      "outcome": "still-open",
      "reason": "The implementation now explicitly records a generated occurrence at a model alias declaration span. FR-093:92-101 and FR-094:127-131 remain unchanged, and the frozen delta contains no ruling making an unreferenced imported relationship a root or defining its model-selection source occurrence. The requested no-functions, reordered-functions and multiple-model provenance assertions are also absent. Record the paired scoped rule and verify those cases."
    }
  ],
  "new_findings": [],
  "qualification": "Static delta review only. Four provided pre-push logs each report this exact head and exit=0 (fmt, clippy, string-edge, check-tests); these are compilation/style checks, not runtime corpus admission receipts. git diff --check passed. RelationshipModel remains wired through production emit_checked and real IR admission; no build or test was run by reviewer."
}
```

## Provenance ruling disposition

The selected model declaration is the scoped source occurrence for an imported
relationship that no checked expression or frame names. The lowering records one
`generated` occurrence at that declaration span; it does not derive a location
from function order, a function body, or generated root. This preserves FR-056's
source span and gives FR-093/FR-094's model-owned relationship node a resolvable
occurrence.

The focused lowering coverage exercises a selected relationship with a function,
while the production location map is keyed by model alias and therefore does not
depend on function presence or ordering. The corpus admission test asserts the
emitted relationship family is admitted through the checked-package/v2 reader.

## Disposition pass 2

FND-001 is resolved at the follow-up head. The scoped rule is now explicit in
FR-056: relationship declarations use FR-154's package-rooted relationship
slot, while fields, operations and operation parameters use owner/name. The
lowering records one generated occurrence at the selected model declaration
span, independent of function presence or ordering. Focused lowering, canonical
intake, owner/name refusal, the pinned FCD golden, and the admission corpus all
pass on the follow-up working tree.
