---
id: SR-4969
title: "QSL-359 code-review"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@8509dcc41150bc42f926981877b635fd63726fb4; relationship lowering; admission corpus; FR-093; FR-094"
review_set: subset
---

## Summary

QSL-359 / PR #730: FAIL. The diff selects relationship records and moves the corpus family row, but test compilation and production source-occurrence placement prevent acceptance; unconditional relationship provenance also lacks a scoped normative decision.

## Verdict

FAIL — findings require disposition.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The newly added test calls relationship_nodes with no Location argument | qsl-semantics/src/check/lowering/model/tests.rs:601 |
| FND-002 | high | In the added RelationshipModel fixture the relationship is not named by a checked expression/frame | qsl-semantics/src/check/lowering/model.rs:375 |

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
      "severity": "high",
      "confidence": "high",
      "method": "code-review",
      "check": "compilation",
      "artifact_id": "selected_relationships_are_rooted_in_the_checked_graph",
      "related_ids": [
        "QSL-359"
      ],
      "path": "qsl-semantics/src/check/lowering/model/tests.rs",
      "lines": "601",
      "excerpt": "|lowering| lowering.relationship_nodes(),",
      "finding": "The newly added test calls relationship_nodes with no Location argument. The sole method at model.rs:364 requires &Location; lower passes &mut Lowering directly, without an adapter. Compiling the semantics unit tests therefore fails with missing argument E0061. Pass a valid fixture location and retain an assertion on the resulting relation node. Static signature/call trace, no build run."
    },
    {
      "fnd": "FND-002",
      "severity": "high",
      "confidence": "high",
      "method": "code-review",
      "check": "soundness",
      "artifact_id": "relationship_nodes",
      "related_ids": [
        "QSL-359"
      ],
      "path": "qsl-semantics/src/check/lowering/model.rs",
      "lines": "375-376",
      "excerpt": "self.model_node(&declaration, &location)?;",
      "finding": "In the added RelationshipModel fixture the relationship is not named by a checked expression/frame. model_node inserts content and correspondence but no occurrence; insert_node uses location only for metering/errors. No graph path reaches this relationship. Lowering::finish (lowering.rs:1400-1406) therefore assigns the fallback generated_location passed at check/mod.rs:1453, i.e. Origin::Expression. region.rs:104-105 cannot resolve it, and emit.rs:769-775 returns UnlocatedOccurrence. The new corpus fixture uses real emit_checked through Emitted::placed, so it fails before the IR reader and the admitted row is not established. Implement an authorized source-occurrence placement for the rooted relationship, then assert actual emission/admission and the placement; supplying a first-function error location alone does not place a source-map occurrence."
    }
  ],
  "bindings": [],
  "qualification": "Diff-scoped static Rust/code and acceptance review only; no production edits, agents, builds or aggregate matrix. git diff --check passed. No runtime gate claim. Plan completion: not assessed."
}
```

## Methods and limits

The Rust lane is folded into code-review. Gap assessment is restricted to changed behavior and its stated FR/TC owners, using source/spec comparison; no repository-wide computed matrix was requested or claimed. No specification text changed and no applicable AssuranceProfile was found. Plan completion: not assessed.

## Dispositions

Round 1, reviewed 29b81ae6d6bb07568e5fa250e3e27c4db5ac8a74. Current verdict: FAIL.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 29b81ae6d6bb07568e5fa250e3e27c4db5ac8a74 |
| FND-002 | fixed | 29b81ae6d6bb07568e5fa250e3e27c4db5ac8a74 |
| FND-003 | still-open | The fix admits ix://<package>/relationship/<name> in addition to owner/name, although unchanged FR-056:408 requires every member identity to be its owner identity, slash and member name. The existing negative test is replaced with a positive test using the formerly refused input (intake.rs:4349). This expands a normative malformed-declaration boundary without a specification/contract decision. Resolve the current upstream relationship-identity contract and align FR-056 and its refusal tests before admitting both forms; do not retain an undocumented compatibility path merely to make the new fixture pass. |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-003 | medium | Relationship intake admits an identity form excluded by unchanged FR-056 | qsl-semantics/src/model/intake.rs:2535; spec/functional/FR-056-admit-domain-package-model-declarations.md:408 |

## Delta evidence

```yaml
{
  "reviewed": "29b81ae6d6bb07568e5fa250e3e27c4db5ac8a74",
  "dispositions": [
    {
      "fnd": "FND-001",
      "outcome": "fixed",
      "fix_sha": "29b81ae6d6bb07568e5fa250e3e27c4db5ac8a74",
      "after_excerpt": "|lowering| {\n            lowering.relationship_nodes(&BTreeMap::from([(\"M\".to_owned(), generated_location())]))\n        },"
    },
    {
      "fnd": "FND-002",
      "outcome": "fixed",
      "fix_sha": "29b81ae6d6bb07568e5fa250e3e27c4db5ac8a74",
      "after_excerpt": "self.record(key, OccurrenceRole::Generated, location);"
    },
    {
      "fnd": "FND-003",
      "outcome": "still-open",
      "reason": "The fix admits ix://<package>/relationship/<name> in addition to owner/name, although unchanged FR-056:408 requires every member identity to be its owner identity, slash and member name. The existing negative test is replaced with a positive test using the formerly refused input (intake.rs:4349). This expands a normative malformed-declaration boundary without a specification/contract decision. Resolve the current upstream relationship-identity contract and align FR-056 and its refusal tests before admitting both forms; do not retain an undocumented compatibility path merely to make the new fixture pass."
    }
  ],
  "new_findings": [
    {
      "fnd": "FND-003",
      "severity": "medium",
      "confidence": "high",
      "method": "code-review",
      "check": "contract-conformance",
      "artifact_id": "read_relationship",
      "path": "qsl-semantics/src/model/intake.rs",
      "lines": "2535-2546",
      "related_ids": [
        "FR-056",
        "TC-145",
        "QSL-359"
      ],
      "excerpt": "let valid_identity = member_identity_name(owner_identity, node).is_some()\n        || node\n            .strip_prefix(&canonical_prefix)\n            .is_some_and(|name| !name.is_empty() && !name.contains('/'));",
      "finding": "The fix admits ix://<package>/relationship/<name> in addition to owner/name, although unchanged FR-056:408 requires every member identity to be its owner identity, slash and member name. The existing negative test is replaced with a positive test using the formerly refused input (intake.rs:4349). This expands a normative malformed-declaration boundary without a specification/contract decision. Resolve the current upstream relationship-identity contract and align FR-056 and its refusal tests before admitting both forms; do not retain an undocumented compatibility path merely to make the new fixture pass."
    }
  ],
  "qualification": "Static delta review only. Four provided pre-push logs each report this exact head and exit=0 (fmt, clippy, string-edge, check-tests); these are compilation/style checks, not runtime corpus admission receipts. git diff --check passed. RelationshipModel remains wired through production emit_checked and real IR admission; no build or test was run by reviewer."
}
```

## Disposition pass 2

FND-003 is fixed at the follow-up head. FR-056 now cites FR-154:117-126 and
FR-154-AC-9 for the relationship-slot identity, with FR-322:665 recorded as
the downstream canonical-key authority. `read_relationship` admits only the
package-rooted `ix://<package>/relationship/<Name>-<verb>-<TargetName>` form;
the owner/name form remains for fields and operations. The focused canonical
positive test and owner/name refusal test both pass.
